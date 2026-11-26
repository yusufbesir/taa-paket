// TAA teslim onayı: karekod sayfasında (/o/<t>) parmakla imza. Cloudflare ve Vercel'in ORTAK kodu (./kopyala.sh kopyalar).
// Yusuf 2026-11-26 06:23: "telefon üzerinden de imza alınsın dokunmatik ekran ile — fişi yanında olmayan için".
// Sayfa önceden betiksizdi; imza için tek küçük betik gerekir. Betik satır içi DEĞİLDİR: ayrı statik yoldan
// (/o/imza.js) sunulur ve yalnız imzalı form sayfasının CSP'si "script-src 'self'" ile açılır (diğer sayfalar betiksiz).
// Betiğin kaynağı aşağıdaki iki gerçek işlevdir (imzaIstemci, griPng); Function.prototype.toString ile metne çevrilir:
// böylece betik hem sözdizimi denetiminden geçer hem de Node'da sahte DOM ile sınanır (test/dene7.mjs).
// İmza: 600×240 tuval, beyaz zemin, koyu çizgi → 8 bit GRİ TONLAMALI PNG (CompressionStream "deflate" = zlib); tarayıcıda
// CompressionStream yoksa tuval.toDataURL (RGBA PNG) yedeği. Sonuç IMZA_EN_COK baytı geçerse önce 4 gri düzeye
// indirilir, yine geçerse yarı çözünürlüğe (300×120) düşürülür. Bulut PNG'yi yeniden doğrular: imzaDogrula (biçim) ve
// imzaIcerik (PNG gerçekten çözülür: CRC, IDAT boyu, tRNS saydamlığı beyaz zemine, koyu piksel kuralı; boş tuval / tek nokta /
// kara tuval / saydam "mürekkep" reddedilir).
// Aynı içerik kuralı mağaza sunucusunda da vardır (crates/taa-fis-sunucu/src/imza_png.rs); asıl karar sunucunundur.
// Konsola hiçbir şey yazılmaz. Yazan: [Claude Opus 5.5, TerziTakip başlığı, 2026-11-26].

/** Çözülmüş PNG en çok bu kadar bayt olabilir (~60 KB). */
export const IMZA_EN_COK = 61440;
/** PNG en/boy üst sınırı (istemci 600×240 üretir; pay bırakıldı). */
export const IMZA_EN = 600;
export const IMZA_BOY = 240;
export const IMZA_EN_SINIR = 800;
export const IMZA_BOY_SINIR = 400;
/** /o/<t> POST gövdesinin üst sınırı (bayt): base64 imza (≤ 81 920) + yüzde kodlama payı + diğer alanlar. */
export const FORM_EN_COK = 131072;
export const IMZA_BETIK_YOLU = "/o/imza.js";
const ONEK = "data:image/png;base64,";
const B64_EN_COK = Math.ceil(IMZA_EN_COK / 3) * 4; // 81 920
const PNG_IMZA = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
const IEND = [0, 0, 0, 0, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82];

/**
 * Formdaki gizli "imza" alanını doğrular. Kabul: "data:image/png;base64,<b64>", çözülmüş ≤ IMZA_EN_COK bayt, PNG imzası,
 * ilk parça IHDR, 1 ≤ en ≤ 800, 1 ≤ boy ≤ 400, son parça IEND. Dönüş: {b64} (önek olmadan) | {hata: "yok"|"buyuk"|"gecersiz"}.
 * (Renk türü serbesttir: gri tonlama asıl yoldur, CompressionStream'siz tarayıcının RGBA yedeği de geçer.)
 */
export function imzaDogrula(deger) {
  if (deger == null || deger === "") return { hata: "yok" };
  if (typeof deger !== "string" || !deger.startsWith(ONEK)) return { hata: "gecersiz" };
  const b64 = deger.slice(ONEK.length);
  if (b64.length > B64_EN_COK) return { hata: "buyuk" };
  if (b64.length < 80 || b64.length % 4 !== 0 || !/^[A-Za-z0-9+/]+={0,2}$/.test(b64)) return { hata: "gecersiz" };
  let ikili;
  try {
    ikili = atob(b64);
  } catch {
    return { hata: "gecersiz" };
  }
  if (ikili.length > IMZA_EN_COK) return { hata: "buyuk" };
  const b = (i) => ikili.charCodeAt(i);
  const u32 = (i) => ((b(i) << 24) >>> 0) + (b(i + 1) << 16) + (b(i + 2) << 8) + b(i + 3);
  if (ikili.length < 57 || PNG_IMZA.some((v, i) => b(i) !== v)) return { hata: "gecersiz" };
  if (u32(8) !== 13 || ikili.slice(12, 16) !== "IHDR") return { hata: "gecersiz" };
  const en = u32(16);
  const boy = u32(20);
  if (en < 1 || en > IMZA_EN_SINIR || boy < 1 || boy > IMZA_BOY_SINIR) return { hata: "gecersiz" };
  const s = ikili.length - 12;
  if (IEND.some((v, i) => b(s + i) !== v)) return { hata: "gecersiz" };
  return { b64 };
}

/** İçerik kuralı (imza_png.rs ile aynı): koyu = beyaz zemine karıştırılmış parlaklık < 128. */
export const IMZA_EN_AZ_KOYU = 60;
let CRC_TABLO = null;
function crc32(b, bas, son) {
  if (!CRC_TABLO) {
    CRC_TABLO = new Uint32Array(256);
    for (let n = 0; n < 256; n++) {
      let c = n;
      for (let j = 0; j < 8; j++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      CRC_TABLO[n] = c >>> 0;
    }
  }
  let c = 0xffffffff;
  for (let i = bas; i < son; i++) c = CRC_TABLO[(c ^ b[i]) & 255] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
/** zlib akışını sınırlı açar (DecompressionStream "deflate": Workers, Vercel/Node 18+). Sınır aşılırsa ya da bozuksa null. */
async function zlibAc(veri, sinir) {
  try {
    const okuyucu = new Blob([veri]).stream().pipeThrough(new DecompressionStream("deflate")).getReader();
    const parcalar = [];
    let top = 0;
    for (;;) {
      const { done, value } = await okuyucu.read();
      if (done) break;
      top += value.length;
      if (top > sinir) { okuyucu.cancel().catch(() => {}); return null; }
      parcalar.push(value);
    }
    const sonuc = new Uint8Array(top);
    let yer = 0;
    for (const p of parcalar) { sonuc.set(p, yer); yer += p.length; }
    return sonuc;
  } catch {
    return null;
  }
}

/**
 * İmza PNG'sinin İÇERİK denetimi (b64: imzaDogrula'nın döndürdüğü önek siz base64). PNG gerçekten çözülür: parça CRC'leri,
 * IHDR ilk / IEND son, 8 bit gri/RGB/gri+alfa/RGBA, taramasız, IDAT tam boy, satır süzgeçleri. Kural: koyu piksel ≥ 60 ve
 * ≥ binde 1,5; ≤ %60; koyu sütun aralığı ≥ en/10. Dönüş: {tamam:true, koyu, koyuEn} | {hata}. "Gerçek imza" kanıtı değildir.
 */
export async function imzaIcerik(b64) {
  let ikili;
  try { ikili = atob(b64); } catch { return { hata: "gecersiz" }; }
  const b = new Uint8Array(ikili.length);
  for (let i = 0; i < ikili.length; i++) b[i] = ikili.charCodeAt(i);
  const u32 = (i) => ((b[i] << 24) >>> 0) + (b[i + 1] << 16) + (b[i + 2] << 8) + b[i + 3];
  if (b.length < 45 || PNG_IMZA.some((v, i) => b[i] !== v)) return { hata: "gecersiz" };
  let i = 8, ihdr = null, iend = false, saydam = null;
  const idat = [];
  while (i < b.length) {
    if (iend || b.length - i < 12) return { hata: "gecersiz" };
    const uz = u32(i);
    if (uz > b.length - i - 12) return { hata: "gecersiz" };
    const tur = String.fromCharCode(b[i + 4], b[i + 5], b[i + 6], b[i + 7]);
    if (crc32(b, i + 4, i + 8 + uz) !== u32(i + 8 + uz)) return { hata: "gecersiz" };
    if (tur === "IHDR") {
      if (ihdr || i !== 8 || uz !== 13) return { hata: "gecersiz" };
      ihdr = { en: u32(16), boy: u32(20), derinlik: b[24], renk: b[25], diger: b[26] | b[27] | b[28] };
    } else if (!ihdr) return { hata: "gecersiz" };
    else if (tur === "IDAT") idat.push(b.subarray(i + 8, i + 8 + uz));
    else if (tur === "IEND") iend = true;
    else if (tur === "tRNS") {
      // Saydam renk (imza_png.rs ile aynı): renk 0 → [gri], renk 2 → [r, g, b] (16 bit örnek, 8 bit derinlikte üst bayt 0).
      // Tekrar, IDAT'tan sonra ya da alfalı/başka renk türünde tRNS geçersizdir.
      if (saydam || idat.length) return { hata: "gecersiz" };
      const u16 = (j) => (b[j] << 8) | b[j + 1];
      if (ihdr.renk === 0 && uz === 2) saydam = [u16(i + 8)];
      else if (ihdr.renk === 2 && uz === 6) saydam = [u16(i + 8), u16(i + 10), u16(i + 12)];
      else return { hata: "gecersiz" };
    } else if (tur.charCodeAt(0) < 97) return { hata: "gecersiz" }; // bilinmeyen kritik parça (ör. PLTE)
    i += 12 + uz;
  }
  if (!ihdr || !iend) return { hata: "gecersiz" };
  const { en, boy, derinlik, renk, diger } = ihdr;
  if (en < 1 || en > IMZA_EN_SINIR || boy < 1 || boy > IMZA_BOY_SINIR) return { hata: "gecersiz" };
  if (derinlik !== 8 || ![0, 2, 4, 6].includes(renk) || diger !== 0) return { hata: "gecersiz" };
  const kanal = { 0: 1, 2: 3, 4: 2, 6: 4 }[renk];
  const satir = en * kanal;
  const beklenen = (satir + 1) * boy;
  let topIdat = 0;
  for (const p of idat) topIdat += p.length;
  const z = new Uint8Array(topIdat);
  let yer = 0;
  for (const p of idat) { z.set(p, yer); yer += p.length; }
  const ham = await zlibAc(z, beklenen);
  if (!ham || ham.length !== beklenen) return { hata: "gecersiz" };
  let onceki = new Uint8Array(satir), simdiki = new Uint8Array(satir);
  let koyu = 0, sol = Infinity, sag = -1;
  for (let y = 0; y < boy; y++) {
    const o = y * (satir + 1), suz = ham[o];
    if (suz > 4) return { hata: "gecersiz" };
    for (let x = 0; x < satir; x++) {
      const a = x >= kanal ? simdiki[x - kanal] : 0, u = onceki[x], c = x >= kanal ? onceki[x - kanal] : 0;
      let t = 0;
      if (suz === 1) t = a;
      else if (suz === 2) t = u;
      else if (suz === 3) t = (a + u) >> 1;
      else if (suz === 4) {
        const p = a + u - c, pa = Math.abs(p - a), pb = Math.abs(p - u), pc = Math.abs(p - c);
        t = pa <= pb && pa <= pc ? a : pb <= pc ? u : c;
      }
      simdiki[x] = (ham[o + 1 + x] + t) & 255;
    }
    for (let x = 0; x < en; x++) {
      const k = x * kanal;
      let l, alfa = 255;
      if (renk === 0 || renk === 4) l = simdiki[k];
      else l = Math.floor((simdiki[k] * 299 + simdiki[k + 1] * 587 + simdiki[k + 2] * 114) / 1000);
      if (renk === 4) alfa = simdiki[k + 1];
      else if (renk === 6) alfa = simdiki[k + 3];
      // tRNS: saydam renkle birebir aynı piksel tam saydamdır; beyaz zemin görünür (saydam "mürekkep" koyu sayılmaz).
      if (saydam && (renk === 0 ? simdiki[k] === saydam[0]
        : simdiki[k] === saydam[0] && simdiki[k + 1] === saydam[1] && simdiki[k + 2] === saydam[2])) alfa = 0;
      if (Math.floor((l * alfa + 255 * (255 - alfa)) / 255) < 128) {
        koyu++;
        if (x < sol) sol = x;
        if (x > sag) sag = x;
      }
    }
    const g = onceki; onceki = simdiki; simdiki = g;
  }
  const toplam = en * boy;
  const koyuEn = koyu ? sag - sol + 1 : 0;
  if (koyu < Math.max(IMZA_EN_AZ_KOYU, Math.floor((toplam * 15) / 10000))) return { hata: "bos", koyu, koyuEn };
  if (koyu > Math.floor((toplam * 6) / 10)) return { hata: "bos", koyu, koyuEn };
  if (koyuEn < Math.ceil(en / 10)) return { hata: "bos", koyu, koyuEn };
  return { tamam: true, koyu, koyuEn };
}

/**
 * 8 bit gri tonlamalı PNG kodlayıcı (tarayıcı ve Node 18+; CompressionStream "deflate" = zlib biçimi, PNG IDAT'ın istediği).
 * gri: en*boy bayt (0 siyah, 255 beyaz). Satır süzgeci "Sub" (beyaz zemin sıfıra iner, iyi sıkışır). Dönüş: Uint8Array.
 * DİKKAT: bu işlev toString ile betiğe gömülür; dışarıdaki hiçbir adı kullanmamalı.
 */
export async function griPng(gri, en, boy) {
  var satir = en + 1;
  var ham = new Uint8Array(satir * boy);
  for (var y = 0; y < boy; y++) {
    var o = y * satir;
    var k = y * en;
    ham[o] = 1;
    ham[o + 1] = gri[k];
    for (var x = 1; x < en; x++) ham[o + 1 + x] = (gri[k + x] - gri[k + x - 1]) & 255;
  }
  var akim = new Blob([ham]).stream().pipeThrough(new CompressionStream("deflate"));
  var z = new Uint8Array(await new Response(akim).arrayBuffer());
  var tablo = new Uint32Array(256);
  for (var n = 0; n < 256; n++) {
    var c = n;
    for (var j = 0; j < 8; j++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    tablo[n] = c >>> 0;
  }
  function crc(b, bas, son) {
    var c = 0xffffffff;
    for (var i = bas; i < son; i++) c = tablo[(c ^ b[i]) & 255] ^ (c >>> 8);
    return (c ^ 0xffffffff) >>> 0;
  }
  function parca(tur, veri) {
    var p = new Uint8Array(12 + veri.length);
    var g = new DataView(p.buffer);
    g.setUint32(0, veri.length);
    for (var i = 0; i < 4; i++) p[4 + i] = tur.charCodeAt(i);
    p.set(veri, 8);
    g.setUint32(8 + veri.length, crc(p, 4, 8 + veri.length));
    return p;
  }
  var ihdr = new Uint8Array(13);
  var d = new DataView(ihdr.buffer);
  d.setUint32(0, en);
  d.setUint32(4, boy);
  ihdr[8] = 8; // bit derinliği
  ihdr[9] = 0; // renk türü: gri
  var parcalar = [new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), parca("IHDR", ihdr), parca("IDAT", z), parca("IEND", new Uint8Array(0))];
  var top = 0;
  parcalar.forEach(function (p) { top += p.length; });
  var sonuc = new Uint8Array(top);
  var yer = 0;
  parcalar.forEach(function (p) { sonuc.set(p, yer); yer += p.length; });
  return sonuc;
}

/**
 * İstemci betiği (tarayıcıda çalışır). png: griPng. sinir: IMZA_EN_COK. Sayfa sözleşmesi (onay.mjs onaySayfa):
 * #imza-alan (data-kisa, data-bos), #imza-tuval (600×240, başta hidden), #imza-veri (gizli input name="imza"),
 * #imza-temizle (başta hidden), #imza-uyari (aria-live). Betik yoksa tuval gizli kalır, <noscript> uyarısı görünür,
 * bulut imzasız gönderimi reddeder. "En az birkaç çizgi": toplam çizgi uzunluğu ≥ 150 px ve (≥ 2 vuruş ya da ≥ 400 px).
 * DİKKAT: toString ile gömülür; dışarıdaki adları kullanmaz (yalnız tarayıcı globalleri ve parametreler).
 */
export function imzaIstemci(png, sinir) {
  "use strict";
  var tuval = document.getElementById("imza-tuval");
  if (!tuval || !tuval.getContext) return;
  var alan = document.getElementById("imza-alan");
  var veri = document.getElementById("imza-veri");
  var temizle = document.getElementById("imza-temizle");
  var uyari = document.getElementById("imza-uyari");
  var form = tuval.closest("form");
  var cx = tuval.getContext("2d");
  var EN = tuval.width;
  var BOY = tuval.height;
  var vurus = 0;
  var uzunluk = 0;
  var son = null;
  var aktif = null;
  var gonderiliyor = false;
  var MUREKKEP = "#111";

  function zemin() {
    cx.fillStyle = "#fff";
    cx.fillRect(0, 0, EN, BOY);
    cx.fillStyle = MUREKKEP;
    cx.strokeStyle = MUREKKEP;
    cx.lineCap = "round";
    cx.lineJoin = "round";
  }
  function goster(metin) {
    uyari.textContent = metin || "";
    uyari.hidden = !metin;
  }
  function nokta(e) {
    var r = tuval.getBoundingClientRect();
    return { x: ((e.clientX - r.left) * EN) / (r.width || EN), y: ((e.clientY - r.top) * BOY) / (r.height || BOY) };
  }
  function kalinlik(e) {
    var k = EN / 150; // 600 px tuvalde 4 px
    if (e.pointerType === "pen" && e.pressure > 0) k *= 0.6 + e.pressure;
    return k;
  }
  function yeterli() {
    return uzunluk >= 150 && (vurus >= 2 || uzunluk >= 400);
  }
  function sifirla() {
    zemin();
    vurus = 0;
    uzunluk = 0;
    son = null;
    aktif = null;
    veri.value = "";
    goster("");
  }

  tuval.addEventListener("pointerdown", function (e) {
    if (aktif !== null || (e.button !== undefined && e.button > 0)) return;
    e.preventDefault();
    if (tuval.setPointerCapture) tuval.setPointerCapture(e.pointerId);
    aktif = e.pointerId;
    son = nokta(e);
    cx.beginPath();
    cx.arc(son.x, son.y, kalinlik(e) / 2, 0, 2 * Math.PI);
    cx.fill();
    vurus++;
    veri.value = "";
    goster("");
  });
  tuval.addEventListener("pointermove", function (e) {
    if (e.pointerId !== aktif || !son) return;
    e.preventDefault();
    var olaylar = e.getCoalescedEvents ? e.getCoalescedEvents() : [];
    if (!olaylar || !olaylar.length) olaylar = [e];
    for (var i = 0; i < olaylar.length; i++) {
      var n = nokta(olaylar[i]);
      cx.lineWidth = kalinlik(olaylar[i]);
      cx.beginPath();
      cx.moveTo(son.x, son.y);
      cx.lineTo(n.x, n.y);
      cx.stroke();
      uzunluk += Math.sqrt((n.x - son.x) * (n.x - son.x) + (n.y - son.y) * (n.y - son.y));
      son = n;
    }
  });
  function bitir(e) {
    if (e.pointerId !== aktif) return;
    aktif = null;
    son = null;
  }
  tuval.addEventListener("pointerup", bitir);
  tuval.addEventListener("pointercancel", bitir);
  tuval.addEventListener("lostpointercapture", bitir);
  temizle.addEventListener("click", sifirla);

  // Tuvali gri tonlamaya çevirir (beyaz zemin + koyu mürekkep: kırmızı kanal yeter); düzey > 0 ise o kadar düzeye indirir.
  function griAl(kaynak, en, boy, duzey) {
    var d = kaynak.getContext("2d").getImageData(0, 0, en, boy).data;
    var g = new Uint8Array(en * boy);
    for (var i = 0; i < g.length; i++) {
      var v = d[i * 4];
      if (duzey) v = Math.round(Math.round((v / 255) * (duzey - 1)) * (255 / (duzey - 1)));
      g[i] = v;
    }
    return g;
  }
  function b64(u) {
    var s = "";
    for (var i = 0; i < u.length; i += 0x8000) s += String.fromCharCode.apply(null, u.subarray(i, i + 0x8000));
    return "data:image/png;base64," + btoa(s);
  }
  function kucuk() {
    var k = document.createElement("canvas");
    k.width = Math.round(EN / 2);
    k.height = Math.round(BOY / 2);
    var kc = k.getContext("2d");
    kc.fillStyle = "#fff";
    kc.fillRect(0, 0, k.width, k.height);
    kc.drawImage(tuval, 0, 0, k.width, k.height);
    return k;
  }
  // ≤ sinir bayt PNG üretir (data URL). Sırayla: tam gri, 4 düzey gri, yarı çözünürlük 4 düzey; yedek toDataURL.
  async function kodla() {
    var denemeler = [[tuval, 0], [tuval, 4], [null, 4]];
    if (typeof CompressionStream === "function") {
      for (var i = 0; i < denemeler.length; i++) {
        var kaynak = denemeler[i][0] || kucuk();
        var p = await png(griAl(kaynak, kaynak.width, kaynak.height, denemeler[i][1]), kaynak.width, kaynak.height);
        if (p.length <= sinir) return b64(p);
      }
      return "";
    }
    var tam = tuval.toDataURL("image/png");
    if (Math.floor(((tam.length - 22) * 3) / 4) <= sinir) return tam;
    var yari = kucuk().toDataURL("image/png");
    return Math.floor(((yari.length - 22) * 3) / 4) <= sinir ? yari : "";
  }

  form.addEventListener("submit", function (e) {
    if (gonderiliyor) {
      e.preventDefault();
      return;
    }
    if (!yeterli()) {
      e.preventDefault();
      goster(vurus === 0 ? alan.getAttribute("data-bos") : alan.getAttribute("data-kisa"));
      if (tuval.focus) tuval.focus();
      return;
    }
    e.preventDefault();
    gonderiliyor = true;
    kodla().then(function (d) {
      if (!d) {
        gonderiliyor = false;
        goster(alan.getAttribute("data-kisa"));
        return;
      }
      veri.value = d;
      form.submit();
    }, function () {
      gonderiliyor = false;
      goster(alan.getAttribute("data-bos"));
    });
  });

  zemin();
  tuval.hidden = false;
  temizle.hidden = false;
}

/** /o/imza.js içeriği: iki işlevin kaynağı, sabit sınırla çağrılır (satır içi betik yok; CSP script-src 'self'). */
// Not: wrangler (esbuild, keepNames açık) iç işlevlerin arkasına "__name(f, ad)" çağrısı ekleyebilir; toString çıktısında bu
// ad tarayıcıda tanımsız kalmasın diye betiğin başında zararsız bir __name tanımı vardır.
export const IMZA_BETIK =
  "// TAA teslim onayı imza betiği (bulut/musteri-takip/ortak/imza.mjs'ten üretilir).\n" +
  "var __name = function (f) { return f; };\n" +
  `(${imzaIstemci.toString()})(${griPng.toString()}, ${IMZA_EN_COK});\n`;
// Paketleyici hedefi düşürürse (async → yardımcı işlev) toString çıktısı tarayıcıda tanımsız ad taşır; o durumda betik
// sunulmaz (503): tuval gizli kalır, i=1 sayfası imzasız gönderimi reddeder, kasiyer mağaza ekranında imza alır.
// Dağıtımdan sonra canlı /o/imza.js yanıtı "node --check" ile sınanmalı (README).
const BETIK_BOZUK = /\b(__async|__awaiter|_asyncToGenerator|__generator|regeneratorRuntime)\b/.test(IMZA_BETIK);

/** GET /o/imza.js yanıtı. */
export function imzaBetikYaniti(yontem = "GET") {
  if (yontem !== "GET" && yontem !== "HEAD") return new Response("yalnız GET", { status: 405 });
  if (BETIK_BOZUK) return new Response("imza betiği bu dağıtımda kullanılamıyor", { status: 503, headers: { "cache-control": "no-store" } });
  return new Response(yontem === "HEAD" ? null : IMZA_BETIK, {
    status: 200,
    headers: {
      "content-type": "text/javascript; charset=utf-8",
      "cache-control": "public, max-age=300",
      "x-content-type-options": "nosniff",
      "content-security-policy": "default-src 'none'; frame-ancestors 'none'",
      "cross-origin-resource-policy": "same-origin",
    },
  });
}
