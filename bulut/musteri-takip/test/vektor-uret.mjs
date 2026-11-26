// Ortak (Rust + JS) sınama vektörlerini üretir: node bulut/musteri-takip/test/vektor-uret.mjs > test/ortak-vektorler.json
// hepsi.mjs bu betiği ÇALIŞTIRMAZ; çıktı dosyası (ortak-vektorler.json) hem dene8.mjs'te hem Rust testlerinde
// (teslim_onay.rs, imza_png.rs; include_str!) okunur. Yalnız sahte veri; ağ gerekmez.
// 1) onay_oz: bulutun GERÇEK kodu (ortak/onay.mjs onayIsle → sifrele, WebCrypto X25519) ile aad = t + ":" + oz.
//    Sunucu X25519 özel anahtarı = 0x11 * 32 (Rust VEKTOR_PK ile aynı); sahte depo, simdi = 1795000000.
// 2) png: tRNS kuralının iki taraftaki karşılığı (beklenen: "tamam" | "bos" | "gecersiz"). PNG kurucu dene8.mjs'tekiyle aynı.
import { createPrivateKey, createPublicKey, generateKeyPairSync, sign } from "node:crypto";
import zlib from "node:zlib";
import { onayIsle } from "../ortak/onay.mjs";
import { imzaIcerik } from "../ortak/imza.mjs";

const pkcs8 = (b) => createPrivateKey({ key: Buffer.concat([Buffer.from("302e020100300506032b656e04220420", "hex"), b]), format: "der", type: "pkcs8" });
const pk = Buffer.from(createPublicKey(pkcs8(Buffer.alloc(32, 0x11))).export({ format: "jwk" }).x, "base64url").toString("hex");
const { publicKey, privateKey } = generateKeyPairSync("ed25519");
const genel = publicKey.export({ format: "der", type: "spki" }).subarray(-32).toString("hex");
const imzala = (g) => sign(null, Buffer.from(g), privateKey).toString("hex");

const SIMDI = 1_795_000_000;
const t = "defghijkmnpqrst23456";
const oz = "5a1e".repeat(16);
const kayitlar = new Map();
const depo = {
  oku: async (x) => kayitlar.get(x) ?? null,
  yaz: async (k) => { kayitlar.set(k.t, k); },
  yazYoksa: async (k) => (kayitlar.has(k.t) ? false : (kayitlar.set(k.t, k), true)),
  sil: async (x) => { kayitlar.delete(x); },
  ekOku: async (x, ad) => kayitlar.get(x + ":" + ad) ?? null,
  ekYazYoksa: async (x, ad, d) => (kayitlar.has(x + ":" + ad) ? false : (kayitlar.set(x + ":" + ad, d), true)),
  ekSil: async (x, ad) => { kayitlar.delete(x + ":" + ad); },
};
const ac = JSON.stringify({ v: 1, tur: "onay_ac", t, ts: SIMDI, sil: SIMDI + 900, s: "MRK01", n: 1, pk, oz });
let r = await onayIsle({ islem: "ac", istek: new Request("https://x/api/onay/ac", { method: "POST", body: ac, headers: { "x-taa-imza": imzala(ac) } }), depo, genel, simdi: SIMDI });
if (r.status !== 200) throw new Error("aç " + r.status);
r = await onayIsle({
  islem: "sayfa", t, depo, genel, simdi: SIMDI,
  istek: new Request("https://x/o/" + t, {
    method: "POST",
    body: new URLSearchParams({ ad: "Deneme Örnekoğlu", tel: "0532 000 45 67", onay: "1", dil: "tr" }).toString(),
    headers: { "content-type": "application/x-www-form-urlencoded", "user-agent": "Mozilla/5.0 (Linux; Android 14; SM-DENEME) Chrome/120", "cf-connecting-ip": "203.0.113.9" },
  }),
});
if (r.status !== 200) throw new Error("onay " + r.status);
const { e, iv, c } = kayitlar.get(t + ":sonuc");

/** PNG kurucu: renk 0 (gri) ya da 2 (RGB), 8 bit; f(x,y) → değer; ekler IDAT'tan önce/sonra. */
function png(en, boy, renk, f, { once = [], sonra = [] } = {}) {
  const kanal = renk === 2 ? 3 : 1;
  const ham = Buffer.alloc((en * kanal + 1) * boy);
  for (let y = 0; y < boy; y++) for (let x = 0; x < en; x++) for (let k = 0; k < kanal; k++) ham[y * (en * kanal + 1) + 1 + x * kanal + k] = f(x, y);
  const parca = (tur, veri) => { const b = Buffer.alloc(12 + veri.length); b.writeUInt32BE(veri.length, 0); b.write(tur, 4, "latin1"); veri.copy(b, 8); b.writeUInt32BE(zlib.crc32(b.subarray(4, 8 + veri.length)), 8 + veri.length); return b; };
  const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(en, 0); ihdr.writeUInt32BE(boy, 4); ihdr[8] = 8; ihdr[9] = renk;
  return Buffer.concat([Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), parca("IHDR", ihdr),
    ...once.map(([tr, v]) => parca(tr, v)), parca("IDAT", zlib.deflateSync(ham, { level: 9 })), ...sonra.map(([tr, v]) => parca(tr, v)), parca("IEND", Buffer.alloc(0))]).toString("base64");
}
const cizgi = (x, y) => (Math.abs(y - 60 - Math.round(20 * Math.sin(x / 10))) <= 1 && x > 30 && x < 270 ? 17 : 255);
const u16 = (...d) => { const b = Buffer.alloc(d.length * 2); d.forEach((v, i) => b.writeUInt16BE(v, i * 2)); return b; };
const pngler = [
  ["gri, tRNS yok", png(300, 120, 0, cizgi), "tamam"],
  ["gri, tRNS = mürekkep (17)", png(300, 120, 0, cizgi, { once: [["tRNS", u16(17)]] }), "bos"],
  ["gri, tRNS = zemin (255)", png(300, 120, 0, cizgi, { once: [["tRNS", u16(255)]] }), "tamam"],
  ["gri, tRNS 16 bit (273) eşleşmez", png(300, 120, 0, cizgi, { once: [["tRNS", u16(273)]] }), "tamam"],
  ["RGB, tRNS = mürekkep", png(300, 120, 2, cizgi, { once: [["tRNS", u16(17, 17, 17)]] }), "bos"],
  ["RGB, tRNS eşleşmeyen", png(300, 120, 2, cizgi, { once: [["tRNS", u16(17, 17, 18)]] }), "tamam"],
  ["gri, tRNS IDAT'tan sonra", png(300, 120, 0, cizgi, { sonra: [["tRNS", u16(17)]] }), "gecersiz"],
  ["gri, tRNS 6 bayt", png(300, 120, 0, cizgi, { once: [["tRNS", u16(17, 17, 17)]] }), "gecersiz"],
];
for (const [ad, b64, beklenen] of pngler) {
  const s = await imzaIcerik(b64);
  if ((s.tamam ? "tamam" : s.hata) !== beklenen) throw new Error(`${ad}: ${JSON.stringify(s)} ≠ ${beklenen}`);
}
console.log(JSON.stringify({
  aciklama: "Sahte sınama vektörleri (test/vektor-uret.mjs üretir; dene8.mjs ve Rust testleri okur). Kişisel veri yok.",
  onay_oz: { ozel: "11".repeat(32), pk, t, oz, e, iv, c, ad: "Deneme Örnekoğlu", tel: "+905320004567", zaman: SIMDI },
  png: pngler.map(([ad, b64, beklenen]) => ({ ad, b64, beklenen })),
}, null, 1));
