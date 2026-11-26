// v7: karekod sayfasında parmakla imza (Yusuf 2026-11-26 06:23: "telefon üzerinden de imza alınsın dokunmatik ekran ile —
// fişi yanında olmayan için"). Aç gövdesinde i=1 → imza zorunlu; imza PNG'si şifreli yükün (P) "imza" alanına girer.
// Sunulan /o/imza.js metni sahte bir DOM'da (çizim yapan küçük tuval taklidi) gerçekten çalıştırılır. Yalnız sahte veri.
import { generateKeyPairSync, sign, createPublicKey, diffieHellman, hkdfSync, createDecipheriv } from "node:crypto";
import zlib from "node:zlib";
import fs from "node:fs";
import assert from "node:assert";
import worker from "../cloudflare/worker.mjs";
import { ONAY_METIN, IMZA_CSP, onaySayfa } from "../ortak/onay.mjs";
import { IMZA_BETIK, IMZA_EN_COK, FORM_EN_COK, IMZA_BETIK_YOLU, imzaDogrula, imzaIcerik, griPng } from "../ortak/imza.mjs";
import { govdeOku } from "../ortak/onay.mjs";
import { DILLER } from "../ortak/diller.mjs";
import { onayDeposuKur, onayYolu, vercelOnay } from "../vercel/lib/onay-depo.mjs";

const { publicKey, privateKey } = generateKeyPairSync("ed25519");
const pubHex = publicKey.export({ format: "der", type: "spki" }).subarray(-32).toString("hex");
const x = generateKeyPairSync("x25519");
const pkHex = x.publicKey.export({ format: "der", type: "spki" }).subarray(-32).toString("hex");
const kv = new Map();
const env = {
  GENEL_ANAHTAR: pubHex,
  TAKIP: {
    async get(k) { const v = kv.get(k); if (!v || (v.exp && v.exp <= Date.now() / 1000)) return null; return JSON.parse(v.deger); },
    async put(k, deger, sec = {}) { kv.set(k, { deger, exp: sec.expiration }); },
    async delete(k) { kv.delete(k); },
  },
};
const simdi = () => Math.floor(Date.now() / 1000);
const imzala = (g) => sign(null, Buffer.from(g), privateKey).toString("hex");
const iste = (yol, sec = {}) => worker.fetch(new Request("https://x" + yol, sec), env);
const ac = (nesne) => { const g = JSON.stringify(nesne); return iste("/api/onay/ac", { method: "POST", body: g, headers: { "x-taa-imza": imzala(g) } }); };
const oku = (t) => { const ts = String(simdi()); return iste("/api/onay/" + t, { headers: { "x-taa-zaman": ts, "x-taa-imza": imzala(`onay_oku:${t}:${ts}`) } }); };
const gonder = (t, alanlar, basliklar = {}) => iste("/o/" + t, { method: "POST", body: new URLSearchParams(alanlar).toString(),
  headers: { "content-type": "application/x-www-form-urlencoded", ...basliklar } });
const ALFABE = "abcdefghjkmnpqrstuvwxyz23456789";
const yeniT = () => Array.from({ length: 20 }, () => ALFABE[Math.floor(Math.random() * ALFABE.length)]).join("");
const acGovde = (t, ek = {}) => ({ v: 1, tur: "onay_ac", t, ts: simdi(), sil: simdi() + 900, s: "MRK01", n: 2, pk: pkHex, ...ek });
function coz(t, { e, iv, c }) {
  const eB = Buffer.from(e, "hex");
  const gen = createPublicKey({ key: Buffer.concat([Buffer.from("302a300506032b656e032100", "hex"), eB]), format: "der", type: "spki" });
  const anahtar = Buffer.from(hkdfSync("sha256", diffieHellman({ privateKey: x.privateKey, publicKey: gen }), Buffer.concat([eB, Buffer.from(pkHex, "hex")]), Buffer.from("taa-teslim-onay-v1"), 32));
  const cB = Buffer.from(c, "base64");
  const d = createDecipheriv("aes-256-gcm", anahtar, Buffer.from(iv, "hex"));
  d.setAAD(Buffer.from(t, "utf8"));
  d.setAuthTag(cB.subarray(-16));
  return JSON.parse(Buffer.concat([d.update(cB.subarray(0, -16)), d.final()]).toString("utf8"));
}

/** PNG çözümleyici (yalnız test): parçalar, CRC, IHDR, IDAT'ı açıp gri pikselleri döndürür (yalnız renk türü 0, süzgeç 0/1). */
function pngAc(buf) {
  assert.deepEqual([...buf.subarray(0, 8)], [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a], "PNG imzası");
  let i = 8; const parcalar = [];
  while (i < buf.length) {
    const n = buf.readUInt32BE(i); const tur = buf.toString("latin1", i + 4, i + 8); const veri = buf.subarray(i + 8, i + 8 + n);
    assert.equal(buf.readUInt32BE(i + 8 + n), zlib.crc32(buf.subarray(i + 4, i + 8 + n)), "CRC " + tur);
    parcalar.push([tur, veri]); i += 12 + n;
  }
  assert.deepEqual(parcalar.map((p) => p[0]), ["IHDR", "IDAT", "IEND"]);
  const ihdr = parcalar[0][1];
  const en = ihdr.readUInt32BE(0), boy = ihdr.readUInt32BE(4);
  assert.equal(ihdr[8], 8, "8 bit"); assert.equal(ihdr[9], 0, "gri tonlama");
  const ham = zlib.inflateSync(parcalar[1][1]);
  assert.equal(ham.length, (en + 1) * boy);
  const gri = new Uint8Array(en * boy);
  for (let y = 0; y < boy; y++) {
    const s = ham[y * (en + 1)]; assert(s === 0 || s === 1, "süzgeç");
    for (let xx = 0; xx < en; xx++) {
      const v = ham[y * (en + 1) + 1 + xx];
      gri[y * en + xx] = s === 1 && xx > 0 ? (v + gri[y * en + xx - 1]) & 255 : v;
    }
  }
  return { en, boy, gri };
}

// ---------------------------------------------------------------- sahte DOM (çizen tuval taklidi)
function sahteTuval(en, boy) {
  const px = new Uint8ClampedArray(en * boy * 4);
  let renk = [0, 0, 0]; let yol = []; let kalin = 1;
  const rgb = (s) => (s === "#fff" ? [255, 255, 255] : [17, 17, 17]);
  const disk = (cx, cy, r) => {
    for (let y = Math.max(0, Math.floor(cy - r)); y <= Math.min(boy - 1, Math.ceil(cy + r)); y++)
      for (let xx = Math.max(0, Math.floor(cx - r)); xx <= Math.min(en - 1, Math.ceil(cx + r)); xx++)
        if ((xx - cx) ** 2 + (y - cy) ** 2 <= r * r) { const o = (y * en + xx) * 4; px[o] = renk[0]; px[o + 1] = renk[1]; px[o + 2] = renk[2]; px[o + 3] = 255; }
  };
  const cx = {
    set fillStyle(v) { this._f = v; }, get fillStyle() { return this._f; },
    set strokeStyle(v) { this._s = v; }, get strokeStyle() { return this._s; },
    set lineWidth(v) { kalin = v; }, get lineWidth() { return kalin; }, lineCap: "", lineJoin: "",
    fillRect(x0, y0, w, h) { renk = rgb(this._f); for (let y = y0; y < y0 + h; y++) for (let xx = x0; xx < x0 + w; xx++) { const o = (y * en + xx) * 4; px[o] = renk[0]; px[o + 1] = renk[1]; px[o + 2] = renk[2]; px[o + 3] = 255; } },
    beginPath() { yol = []; }, moveTo(a, b) { yol.push(["m", a, b]); }, lineTo(a, b) { yol.push(["l", a, b]); }, arc(a, b, r) { yol.push(["a", a, b, r]); },
    fill() { renk = rgb(this._f); for (const p of yol) if (p[0] === "a") disk(p[1], p[2], p[3]); },
    stroke() {
      renk = rgb(this._s);
      for (let k = 1; k < yol.length; k++) {
        const [, a1, b1] = yol[k - 1], [, a2, b2] = yol[k]; const adim = Math.max(1, Math.ceil(Math.hypot(a2 - a1, b2 - b1)));
        for (let s = 0; s <= adim; s++) disk(a1 + ((a2 - a1) * s) / adim, b1 + ((b2 - b1) * s) / adim, kalin / 2);
      }
    },
    getImageData(x0, y0, w, h) { assert(w === en && h === boy); return { data: px }; },
    drawImage(kaynak, x0, y0, w, h) {
      const k = kaynak._px, ke = kaynak.width, kb = kaynak.height;
      for (let y = 0; y < h; y++) for (let xx = 0; xx < w; xx++) {
        const sx = Math.floor((xx * ke) / w), sy = Math.floor((y * kb) / h); const so = (sy * ke + sx) * 4, o = (y * w + xx) * 4;
        px[o] = k[so]; px[o + 1] = k[so + 1]; px[o + 2] = k[so + 2]; px[o + 3] = 255;
      }
    },
  };
  return { width: en, height: boy, _px: px, getContext: () => cx };
}
function sahteSayfa(cssEn = 300) {
  const dinle = (o) => { o._d = {}; o.addEventListener = (ad, f) => { (o._d[ad] ||= []).push(f); }; o.yay = (ad, e = {}) => { for (const f of o._d[ad] || []) f(e); }; return o; };
  const form = dinle({ gonderildi: 0, submit() { this.gonderildi++; } });
  const tuval = dinle(Object.assign(sahteTuval(600, 240), {
    hidden: true, yakalanan: null, setPointerCapture(id) { this.yakalanan = id; }, focus() {}, closest: (s) => (s === "form" ? form : null),
    getBoundingClientRect: () => ({ left: 10, top: 20, width: cssEn, height: cssEn * 0.4 }),
  }));
  const el = {
    "imza-tuval": tuval,
    "imza-alan": { getAttribute: (a) => ({ "data-bos": "BOS", "data-kisa": "KISA" })[a] },
    "imza-veri": { value: "" },
    "imza-temizle": dinle({ hidden: true }),
    "imza-uyari": { textContent: "", hidden: true },
  };
  globalThis.document = { getElementById: (id) => el[id] || null };
  // createElement'ten dönen tuvalin boyutu betikte atanır: width/height atanınca piksel tamponu yeniden kurulur.
  globalThis.document.createElement = (tur) => {
    assert.equal(tur, "canvas");
    const k = { _w: 1, _h: 1 };
    let ic = sahteTuval(1, 1);
    Object.defineProperties(k, {
      width: { get: () => k._w, set: (v) => { k._w = v; ic = sahteTuval(k._w, k._h); } },
      height: { get: () => k._h, set: (v) => { k._h = v; ic = sahteTuval(k._w, k._h); } },
      _px: { get: () => ic._px },
    });
    k.getContext = () => ic.getContext();
    return k;
  };
  new Function(IMZA_BETIK)(); // sunulan betik metni aynen çalışır
  return { form, tuval, el };
}
const olay = (ek) => ({ pointerId: 7, pointerType: "touch", button: 0, pressure: 0.5, preventDefault() { this.engellendi = true; }, ...ek });
/** CSS koordinatında (tuval 300×120 görünür) bir vuruş çizer. */
function vurus(tuval, noktalar) {
  tuval.yay("pointerdown", olay({ clientX: 10 + noktalar[0][0], clientY: 20 + noktalar[0][1] }));
  for (const [a, b] of noktalar.slice(1)) tuval.yay("pointermove", olay({ clientX: 10 + a, clientY: 20 + b }));
  tuval.yay("pointerup", olay({}));
}
async function gonderBekle(form, el) {
  const e = { preventDefault() { this.engellendi = true; } };
  form.yay("submit", e);
  for (let i = 0; i < 200 && !form.gonderildi && el["imza-uyari"].hidden; i++) await new Promise((r) => setTimeout(r, 5));
  return e;
}

let gecti = 0;
/** RGBA PNG kurucu (yalnız test): f(x,y) gri değer; satırlar sırayla None/Sub/Up/Average/Paeth süzgeciyle. */
function rgbaPng(en, boy, f) {
  const satir = en * 4, ham = Buffer.alloc((satir + 1) * boy);
  let onceki = Buffer.alloc(satir);
  for (let y = 0; y < boy; y++) {
    const px = Buffer.alloc(satir);
    for (let x2 = 0; x2 < en; x2++) { const v = f(x2, y); px[x2 * 4] = px[x2 * 4 + 1] = px[x2 * 4 + 2] = v; px[x2 * 4 + 3] = 255; }
    const s2 = y % 5, o = y * (satir + 1); ham[o] = s2;
    for (let i = 0; i < satir; i++) {
      const a = i >= 4 ? px[i - 4] : 0, u = onceki[i], c = i >= 4 ? onceki[i - 4] : 0;
      let t = 0;
      if (s2 === 1) t = a; else if (s2 === 2) t = u; else if (s2 === 3) t = (a + u) >> 1;
      else if (s2 === 4) { const p = a + u - c, pa = Math.abs(p - a), pb = Math.abs(p - u), pc = Math.abs(p - c); t = pa <= pb && pa <= pc ? a : pb <= pc ? u : c; }
      ham[o + 1 + i] = (px[i] - t) & 255;
    }
    onceki = px;
  }
  const parca = (tur, veri) => { const b = Buffer.alloc(12 + veri.length); b.writeUInt32BE(veri.length, 0); b.write(tur, 4, "latin1"); veri.copy(b, 8); b.writeUInt32BE(zlib.crc32(b.subarray(4, 8 + veri.length)), 8 + veri.length); return b; };
  const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(en, 0); ihdr.writeUInt32BE(boy, 4); ihdr[8] = 8; ihdr[9] = 6;
  return Buffer.concat([Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), parca("IHDR", ihdr), parca("IDAT", zlib.deflateSync(ham)), parca("IEND", Buffer.alloc(0))]);
}

const t_ = async (ad, f) => { try { await f(); } catch (e) { e.message = `[${ad}] ${e.message}`; throw e; } gecti++; };

// ---------------------------------------------------------------- testler
let imzaUrl = "";
await t_("istemci betiği (sahte DOM): yetersiz imza engellenir, temizle sıfırlar, yeterli imza gri PNG olarak gönderilir", async () => {
  const { form, tuval, el } = sahteSayfa();
  assert.equal(tuval.hidden, false, "betik tuvali gösterir"); assert.equal(el["imza-temizle"].hidden, false);
  let e = await gonderBekle(form, el);
  assert(e.engellendi && !form.gonderildi); assert.equal(el["imza-uyari"].textContent, "BOS"); assert.equal(el["imza-uyari"].hidden, false);
  vurus(tuval, [[20, 60], [40, 62]]); // tek kısa vuruş
  assert.equal(tuval.yakalanan, 7, "setPointerCapture");
  e = await gonderBekle(form, el);
  assert(e.engellendi && !form.gonderildi); assert.equal(el["imza-uyari"].textContent, "KISA");
  el["imza-temizle"].yay("click");
  assert.equal(el["imza-uyari"].hidden, true); assert.equal(el["imza-veri"].value, "");
  vurus(tuval, [[30, 80], [50, 30], [70, 85], [90, 35], [110, 80]]);
  vurus(tuval, [[130, 60], [170, 50], [210, 70], [260, 55]]);
  e = await gonderBekle(form, el);
  assert(e.engellendi, "asıl gönderim betikten (kodlama sonrası) yapılır");
  assert.equal(form.gonderildi, 1, "form.submit çağrıldı");
  imzaUrl = el["imza-veri"].value;
  assert(imzaUrl.startsWith("data:image/png;base64,"));
  const png = Buffer.from(imzaUrl.slice(22), "base64");
  assert(png.length <= IMZA_EN_COK, "≤ 60 KB: " + png.length);
  const { en, boy, gri } = pngAc(png);
  assert.equal(en, 600); assert.equal(boy, 240);
  const koyu = gri.filter((v) => v < 64).length;
  assert(koyu > 500 && koyu < gri.length / 4, "mürekkep pikselleri: " + koyu);
  assert.equal(gri[0], 255, "beyaz zemin");
  // CSS→tuval ölçeği: CSS (30,80) → tuval (60,160) koyu olmalı
  assert(gri[160 * 600 + 60] < 64, "ölçekleme");
  const e2 = await gonderBekle(form, el); // ikinci gönderim engellenir (çift tıklama)
  assert(e2.engellendi); assert.equal(form.gonderildi, 1);
});
await t_("griPng: gürültülü görüntü sınırı aşar, betik 4 gri düzeye iner; imzaDogrula birim", async () => {
  const gurultu = new Uint8Array(600 * 240).map(() => Math.floor(Math.random() * 256));
  const p = await griPng(gurultu, 600, 240);
  assert(p.length > IMZA_EN_COK);
  assert.equal(pngAc(Buffer.from(p)).gri[12345], gurultu[12345]);
  assert.equal(imzaDogrula("data:image/png;base64," + Buffer.from(p).toString("base64")).hata, "buyuk");
  // betiğin küçültme yolu: tuvali gürültüyle doldurup gönder → yine ≤ sınır PNG üretir (4 düzey)
  const { form, tuval, el } = sahteSayfa();
  const px = tuval._px; for (let i = 0; i < px.length; i += 4) px[i] = px[i + 1] = px[i + 2] = Math.random() * 256;
  vurus(tuval, [[10, 10], [290, 110]]); vurus(tuval, [[10, 110], [290, 10]]);
  await gonderBekle(form, el);
  assert.equal(form.gonderildi, 1);
  const b = Buffer.from(el["imza-veri"].value.slice(22), "base64");
  assert(b.length <= IMZA_EN_COK);
  const ac4 = pngAc(b);
  assert.deepEqual([ac4.en, ac4.boy], [600, 240]);
  assert(new Set(ac4.gri).size <= 4, "4 gri düzeye indirildi (en çok 2 bit/piksel ≈ 36 KB; yarı çözünürlük yalnız toDataURL yedeğinde gerekir)");
  // imzaDogrula
  const iyi = imzaUrl;
  assert.equal(imzaDogrula(iyi).b64, iyi.slice(22));
  assert.equal(imzaDogrula("").hata, "yok"); assert.equal(imzaDogrula(null).hata, "yok");
  for (const kotu of ["data:image/jpeg;base64,AAAA", "data:image/png;base64,!!!!", "javascript:1", iyi.slice(0, -4),
    "data:image/png;base64," + Buffer.from("x".repeat(300)).toString("base64")]) assert.equal(imzaDogrula(kotu).hata, "gecersiz", kotu.slice(0, 40));
  // imzaIcerik: gerçek imza geçer; boş tuval, tek nokta, kara tuval, CRC bozuk, yalnız başlıklı sahte PNG geçmez
  assert.equal((await imzaIcerik(iyi.slice(22))).tamam, true);
  const b64 = (u8) => Buffer.from(u8).toString("base64");
  const bos = await griPng(new Uint8Array(600 * 240).fill(255), 600, 240);
  assert.equal((await imzaIcerik(b64(bos))).hata, "bos");
  assert.equal(imzaDogrula("data:image/png;base64," + b64(bos)).b64, b64(bos), "biçim olarak geçerli");
  const nokta = new Uint8Array(600 * 240).fill(255);
  for (let y = 100; y < 110; y++) for (let x2 = 300; x2 < 310; x2++) nokta[y * 600 + x2] = 0;
  assert.equal((await imzaIcerik(b64(await griPng(nokta, 600, 240)))).hata, "bos");
  assert.equal((await imzaIcerik(b64(await griPng(new Uint8Array(600 * 240), 600, 240)))).hata, "bos", "kara tuval");
  const crcBozuk = Buffer.from(iyi.slice(22), "base64"); crcBozuk[crcBozuk.length - 13] ^= 1;
  assert.equal((await imzaIcerik(crcBozuk.toString("base64"))).hata, "gecersiz");
  const sahte = Buffer.concat([Buffer.from(iyi.slice(22), "base64").subarray(0, 33), Buffer.from("DENEME-IMZA-DENEME-IMZA"), Buffer.from(iyi.slice(22), "base64").subarray(-12)]);
  assert.equal(imzaDogrula("data:image/png;base64," + sahte.toString("base64")).b64 !== undefined, true, "eski biçim denetimi sahteyi geçirirdi");
  assert.equal((await imzaIcerik(sahte.toString("base64"))).hata, "gecersiz");
  // RGBA (toDataURL yedeği) ve Paeth/Up süzgeçleri: Node zlib ile kurulan RGBA PNG
  const rgba = rgbaPng(300, 120, (x2, y) => (Math.abs(y - 60 - Math.round(20 * Math.sin(x2 / 10))) <= 1 && x2 > 30 && x2 < 270 ? 10 : 255));
  assert.equal((await imzaIcerik(rgba.toString("base64"))).tamam, true);
  const genis = Buffer.from(iyi.slice(22), "base64"); genis.writeUInt32BE(801, 16); // en 801 (CRC'ye bakılmaz, boyut reddi)
  assert.equal(imzaDogrula("data:image/png;base64," + genis.toString("base64")).hata, "gecersiz");
  const sonEksik = Buffer.from(iyi.slice(22), "base64").subarray(0, -12);
  assert.equal(imzaDogrula("data:image/png;base64," + Buffer.concat([sonEksik, Buffer.alloc(12)]).toString("base64")).hata, "gecersiz", "IEND yok");
});

const T = yeniT();
await t_("aç i=1: kayıtta i=1; sayfada tuval, gizli alan, noscript uyarısı, dış betik; CSP yalnız script-src 'self'", async () => {
  assert.equal((await ac(acGovde(T, { i: 1 }))).status, 200);
  assert.equal(JSON.parse(kv.get("onay:" + T).deger).i, 1);
  const r = await iste("/o/" + T, { headers: { "accept-language": "tr" } });
  const h = await r.text();
  assert.equal(r.status, 200);
  assert.equal(r.headers.get("content-security-policy"), IMZA_CSP);
  assert.match(IMZA_CSP, /script-src 'self';/); assert(!/unsafe-inline'[^;]*;?\s*$/.test(IMZA_CSP.split("script-src")[1].split(";")[0]), "script-src'de unsafe-inline yok");
  assert.match(IMZA_CSP, /frame-ancestors 'none'/);
  assert(h.includes(`<canvas id="imza-tuval" width="600" height="240"`) && h.includes('name="imza" id="imza-veri"'));
  assert(h.includes(`<noscript><p class="alan-hata">${ONAY_METIN.tr.js_gerekli.replace("'", "&#39;")}</p></noscript>`), "noscript uyarısı");
  assert(h.includes(ONAY_METIN.tr.imza) && h.includes(ONAY_METIN.tr.temizle) && h.includes(ONAY_METIN.tr.imza_aciklama));
  const betikler = h.match(/<script[^>]*>[\s\S]*?<\/script>/g) || [];
  assert.deepEqual(betikler, [`<script src="${IMZA_BETIK_YOLU}" defer></script>`], "tek dış betik, satır içi yok");
  assert(!/\son[a-z]+=/i.test(h), "satır içi olay yok");
  assert(h.indexOf('id="imza-tuval"') < h.indexOf('name="onay"'), "imza onay kutusundan önce");
  // i=0 (eski istemci) sayfası betiksiz kalır, CSP değişmez
  const t0 = yeniT();
  assert.equal((await ac(acGovde(t0))).status, 200);
  const r0 = await iste("/o/" + t0);
  const h0 = await r0.text();
  assert(!/<script/i.test(h0) && !h0.includes("imza-tuval"));
  assert(!r0.headers.get("content-security-policy").includes("script-src"));
  // süresi dolmuş sayfa betiksiz
  const rd = await iste("/o/" + yeniT());
  assert(!rd.headers.get("content-security-policy").includes("script-src"));
});
await t_("betik yolu /o/imza.js: Cloudflare ve Vercel; tür, nosniff, yalnız GET; vercel.json sırası", async () => {
  const r = await iste(IMZA_BETIK_YOLU);
  assert.equal(r.status, 200);
  assert.match(r.headers.get("content-type"), /^text\/javascript/);
  assert.equal(r.headers.get("x-content-type-options"), "nosniff");
  assert.equal(await r.text(), IMZA_BETIK);
  assert.doesNotThrow(() => new Function(IMZA_BETIK));
  assert(!IMZA_BETIK.includes("console.") && !IMZA_BETIK.includes("fetch(") && !IMZA_BETIK.includes("XMLHttpRequest"), "betik ağa/konsola gitmez");
  assert.equal((await iste(IMZA_BETIK_YOLU, { method: "POST", body: "x" })).status, 405);
  assert.deepEqual(onayYolu("https://v.invalid/o/imza.js"), { islem: "betik", t: "" });
  assert.deepEqual(onayYolu("https://v.invalid/api/onay?islem=betik"), { islem: "betik", t: "" });
  const depo = { ...onayDeposuKur({}, async () => new Response("")), tablo: async () => null };
  const rv = await vercelOnay(new Request("https://v.invalid/api/onay?islem=betik"), depo, pubHex);
  assert.equal(rv.status, 200); assert.equal(await rv.text(), IMZA_BETIK);
  const v = JSON.parse(fs.readFileSync(new URL("../vercel/vercel.json", import.meta.url), "utf8"));
  const sira = v.rewrites.map((r) => r.source);
  assert.equal(v.rewrites.find((r) => r.source === "/o/imza.js").destination, "/api/onay?islem=betik");
  assert(sira.indexOf("/o/imza.js") < sira.indexOf("/o/:t"), "/o/imza.js, /o/:t'den önce");
});
await t_("i=1: imzasız / bozuk / büyük imza reddedilir (400, h_imza), kayıt bekliyor kalır", async () => {
  const alan = { ad: "Deneme Kişi", tel: "0555 000 00 07", onay: "1", dil: "tr" };
  let r = await gonder(T, alan);
  let h = await r.text();
  assert.equal(r.status, 400);
  assert(h.includes(ONAY_METIN.tr.h_imza) && h.includes('id="h-imza"') && h.includes("imza-kutu hatali"));
  assert(!h.includes(ONAY_METIN.tr.h_ad) && !h.includes(ONAY_METIN.tr.h_kutu), "yalnız imza hatası");
  assert.equal(r.headers.get("content-security-policy"), IMZA_CSP, "hata sayfası da imzalı form");
  r = await gonder(T, { ...alan, imza: "data:image/png;base64,QUJD" }); assert.equal(r.status, 400);
  const buyuk = "data:image/png;base64," + Buffer.alloc(IMZA_EN_COK + 3).toString("base64");
  r = await gonder(T, { ...alan, imza: buyuk }); assert.equal(r.status, 400);
  assert((await r.text()).includes(ONAY_METIN.tr.h_imza));
  // boş beyaz tuval (biçimi doğru PNG) da imza sayılmaz
  const bosPng = Buffer.from(await griPng(new Uint8Array(600 * 240).fill(255), 600, 240)).toString("base64");
  r = await gonder(T, { ...alan, imza: "data:image/png;base64," + bosPng }); assert.equal(r.status, 400);
  assert((await r.text()).includes(ONAY_METIN.tr.h_imza));
  const k = JSON.parse(kv.get("onay:" + T).deger);
  assert.equal(k.durum, "bekliyor"); assert.equal([...kv.keys()].filter((a) => a.startsWith(`onay:${T}:deneme:`)).length, 4);
  // gövde sınırı: FORM_EN_COK'u aşan gövde okunmaz (imza hatası)
  const t4 = yeniT();
  assert.equal((await ac(acGovde(t4, { i: 1 }))).status, 200);
  r = await gonder(t4, { ...alan, imza: "data:image/png;base64," + "A".repeat(FORM_EN_COK) });
  h = await r.text();
  assert.equal(r.status, 400); assert(h.includes(ONAY_METIN.tr.h_imza));
  assert.equal(JSON.parse(kv.get("onay:" + t4).deger).durum, "bekliyor");
  // content-length başlığı olmayan akış gövdesi de sınırda kesilir (tümü okunmaz)
  let okunan = 0;
  const akim = new ReadableStream({ pull(c) { okunan += 16384; if (okunan > FORM_EN_COK * 8) c.close(); else c.enqueue(new Uint8Array(16384).fill(65)); } });
  const ist = new Request("https://x/o/" + t4, { method: "POST", body: akim, duplex: "half" });
  assert.equal(ist.headers.get("content-length"), null);
  assert.equal(await govdeOku(ist, FORM_EN_COK), null);
  assert(okunan <= FORM_EN_COK + 3 * 16384, "akış erken kesildi: " + okunan);
  assert.equal(await govdeOku(new Request("https://x/", { method: "POST", body: "a=1&b=2" }), FORM_EN_COK), "a=1&b=2");
});
await t_("i=1 imzalı onay: 200; çözülen P = {ad, tel, ua, ip, zaman, dil, imza}, imza PNG birebir; kayıtta düz metin yok", async () => {
  const r = await gonder(T, { ad: "Deneme Kişi", tel: "0555 000 00 07", onay: "1", dil: "tr", imza: imzaUrl },
    { "user-agent": "Sahte/1.0", "cf-connecting-ip": "203.0.113.9" });
  assert.equal(r.status, 200, await r.clone().text());
  assert((await r.text()).includes(ONAY_METIN.tr.tesekkur_b));
  const ham = kv.get(`onay:${T}:sonuc`).deger;
  const k = JSON.parse(ham);
  assert.deepEqual(Object.keys(k).sort(), ["c", "e", "iv", "sil", "t", "yazan"]);
  assert(!ham.includes(imzaUrl.slice(30, 60)) && !ham.includes("Deneme") && !ham.includes("iVBOR"), "imza/ad düz metin değil");
  const j = await (await oku(T)).json();
  const P = coz(T, j);
  assert.deepEqual(Object.keys(P), ["ad", "tel", "ua", "ip", "zaman", "dil", "imza"]);
  assert.equal(P.imza, imzaUrl.slice(22), "base64 PNG, önek yok");
  assert.equal(P.ad, "Deneme Kişi"); assert.equal(P.tel, "+905550000007"); assert.equal(P.ip, "203.0.113.9");
  assert(Buffer.from(P.imza, "base64").length <= IMZA_EN_COK);
  pngAc(Buffer.from(P.imza, "base64"));
  // en büyük kabul edilen imzayla oku yanıtının üst sınırı (sunucu tarafı okuma sınırı için): ≤ 128 KiB
  assert(JSON.stringify(j).length < 131072);
});
await t_("i=0: imza isteğe bağlı; gelirse doğrulanıp yüke girer, bozuksa reddedilir", async () => {
  const t = yeniT();
  assert.equal((await ac(acGovde(t, { i: 0 }))).status, 200);
  assert.equal((await gonder(t, { ad: "Deneme Kişi", tel: "05550000008", onay: "1", imza: "data:image/png;base64,QUJD" })).status, 400);
  assert.equal((await gonder(t, { ad: "Deneme Kişi", tel: "05550000008", onay: "1", imza: imzaUrl })).status, 200);
  assert.equal(coz(t, await (await oku(t)).json()).imza, imzaUrl.slice(22));
});
await t_("en büyük imza (~60 KB) uçtan uca geçer: gövde, KV ve şifreli yük sınırları yeterli", async () => {
  // Gerçek, çözülebilir ~60 KB PNG: 600×100 gri, sıkıştırmasız zlib (level 0) → IMZA_EN_COK'a çok yakın; imza çizgili.
  // Zemin açık gri gürültü (130..255, koyu sayılmaz): base64 metni gerçekçi (yüzde kodlaması rastgele dağılır).
  const en = 600, boy = 100, ham = Buffer.alloc((en + 1) * boy);
  let tohum = 12345; for (let i = 0; i < ham.length; i++) { tohum = (tohum * 1103515245 + 12345) >>> 0; ham[i] = 130 + (tohum >>> 24) % 126; }
  for (let y = 0; y < boy; y++) { ham[y * (en + 1)] = 0; for (let x2 = 60; x2 < 540; x2++) if (Math.abs(y - 50 - Math.round(25 * Math.sin(x2 / 15))) <= 1) ham[y * (en + 1) + 1 + x2] = 0; }
  const parca = (tur, veri) => { const b = Buffer.alloc(12 + veri.length); b.writeUInt32BE(veri.length, 0); b.write(tur, 4, "latin1"); veri.copy(b, 8); b.writeUInt32BE(zlib.crc32(b.subarray(4, 8 + veri.length)), 8 + veri.length); return b; };
  const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(en, 0); ihdr.writeUInt32BE(boy, 4); ihdr[8] = 8;
  const png = Buffer.concat([Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), parca("IHDR", ihdr), parca("IDAT", zlib.deflateSync(ham, { level: 0 })), parca("IEND", Buffer.alloc(0))]);
  assert(png.length <= IMZA_EN_COK && png.length > IMZA_EN_COK - 2048, "≈ 60 KB: " + png.length);
  assert.equal((await imzaIcerik(png.toString("base64"))).tamam, true);
  const url = "data:image/png;base64," + png.toString("base64");
  assert(new URLSearchParams({ imza: url }).toString().length < FORM_EN_COK, "yüzde kodlamayla gövde sınırın altında");
  const t = yeniT();
  assert.equal((await ac(acGovde(t, { i: 1 }))).status, 200);
  const r = await gonder(t, { ad: "Deneme Kişi", tel: "05550000009", onay: "1", imza: url }, { "user-agent": "x".repeat(400) });
  assert.equal(r.status, 200);
  const j = await (await oku(t)).json();
  assert.equal(coz(t, j).imza, png.toString("base64"));
  assert(JSON.stringify(j).length < 131072, "oku yanıtı < 128 KiB: " + JSON.stringify(j).length);
});
await t_("Vercel: i=1 aç → imzasız red → imzalı onay → oku/çöz (sahte Blob)", async () => {
  const bloblar = new Map();
  const blob = {
    async put(y, g, sec = {}) {
      if (sec.allowOverwrite === false && bloblar.has(y)) throw new Error("BlobAlreadyExistsError");
      bloblar.set(y, { govde: g, uploadedAt: new Date() }); return { url: "https://blob.invalid/" + y };
    },
    async head(y) { if (!bloblar.has(y)) throw new Error("BlobNotFoundError"); return { url: "https://blob.invalid/" + y }; },
    async del(u) { bloblar.delete(u.replace("https://blob.invalid/", "")); },
    async list() { return { blobs: [], hasMore: false }; },
  };
  const getir = async (u) => { const b = bloblar.get(u.replace("https://blob.invalid/", "").replace(/\?v=\d+$/, "")); return b ? new Response(b.govde) : new Response("", { status: 404 }); };
  const depo = { ...onayDeposuKur(blob, getir), tablo: async () => null };
  const v = (yol, sec = {}) => vercelOnay(new Request("https://v.invalid" + yol, sec), depo, pubHex);
  const t = yeniT();
  const g = JSON.stringify(acGovde(t, { i: 1 }));
  assert.equal((await v("/api/onay/ac", { method: "POST", body: g, headers: { "x-taa-imza": imzala(g) } })).status, 200);
  const sayfa = await v(`/api/onay?islem=sayfa&t=${t}`);
  assert.equal(sayfa.headers.get("content-security-policy"), IMZA_CSP);
  const post = (alan) => v(`/o/${t}`, { method: "POST", body: new URLSearchParams(alan).toString(), headers: { "content-type": "application/x-www-form-urlencoded" } });
  assert.equal((await post({ ad: "Deneme Kişi", tel: "05550000010", onay: "1" })).status, 400);
  assert.equal((await post({ ad: "Deneme Kişi", tel: "05550000010", onay: "1", imza: imzaUrl })).status, 200);
  const ts = String(simdi());
  const o = await (await v(`/api/onay/${t}`, { headers: { "x-taa-zaman": ts, "x-taa-imza": imzala(`onay_oku:${t}:${ts}`) } })).json();
  assert.equal(coz(t, o).imza, imzaUrl.slice(22));
});
await t_("13 dil: imza metinleri ve imzalı form sayfası", () => {
  for (const [d] of DILLER) {
    for (const a of ["imza", "imza_aciklama", "temizle", "h_imza", "imza_kisa", "js_gerekli"]) assert(ONAY_METIN[d][a]?.trim(), d + "." + a);
    assert(ONAY_METIN[d].js_gerekli.includes("JavaScript"), d + " js_gerekli 'JavaScript' sözcüğünü içerir");
    const h = onaySayfa("form", { dil: d, t: T, sube: "MRK01", n: 1, imza: 1, hatalar: { imza: true } });
    assert(h.includes("imza-tuval") && h.includes("<noscript>") && !h.includes("undefined"), d);
  }
});
console.log(`v7 parmakla imza testleri geçti (${gecti} denetim)`);
