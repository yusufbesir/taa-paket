// Sahip: Yusuf. yazan: [Claude#taa-paket-T2 2026-11-05]
// taa-veri.js v1 — kaynak: yusufbesir/taa-paket/araclar/js/taa-veri.js; programlar kopyalar, elle düzenlemez
'use strict';
// TAA veri alanı kitaplığı (docs/T2-TASARIM.md §1, §1.2). Bağımlılıksız, CommonJS. Hiçbir şey silinmez:
// üzerine yazılan/taşınan dosyalar eski/ altına ya da .tasindi adına yeniden adlandırılır.
const fs = require('fs');
const os = require('os');
const path = require('path');

const ALAN_RE = /^[a-z0-9-]{1,32}$/;
const ONERI_AZAMI = 256 * 1024;
let sayac = 0;

class VeriHata extends Error {
  constructor(kod, ileti, ek) { super(ileti); this.name = kod; this.kod = kod; Object.assign(this, ek || {}); }
}
class BaskaSahip extends VeriHata {
  constructor(alan, sahip) { super('BaskaSahip', `'${alan}' alanının sahibi başka program: ${sahip}`, { alan, sahip }); }
}

// ---- Yollar ----
function veriKok(env) {
  env = env || process.env;
  if (env.TAA_VERI_KOK) return env.TAA_VERI_KOK;
  return path.join(taaKok(env), 'veri');
}
function taaKok(env) {
  if (process.platform === 'win32') return path.join(env.APPDATA || path.join(os.homedir(), 'AppData', 'Roaming'), 'TAA');
  return path.join(env.XDG_DATA_HOME || path.join(env.HOME || os.homedir(), '.local', 'share'), 'TAA');
}
function alanYolu(alan, env) {
  if (typeof alan !== 'string' || !ALAN_RE.test(alan)) throw new VeriHata('GecersizAd', `geçersiz alan adı: ${alan}`);
  env = env || process.env;
  // İstisna: rehber alanı mevcut konumunda kalır (%APPDATA%/TAA/rehber).
  return alan === 'rehber' ? path.join(taaKok(env), 'rehber') : path.join(veriKok(env), alan);
}
function altYol(dizin, ad) {
  if (typeof ad !== 'string' || ad === '' || ad.includes('\0')) throw new VeriHata('YolDisari', `geçersiz dosya adı: ${ad}`);
  if (path.isAbsolute(ad) || path.win32.isAbsolute(ad) || /^[\\/]/.test(ad)) throw new VeriHata('YolDisari', `mutlak yol reddedildi: ${ad}`);
  const parcalar = ad.split(/[\\/]/);
  if (parcalar.some((p) => p === '..' || p === '')) throw new VeriHata('YolDisari', `.. ya da boş bölüm reddedildi: ${ad}`);
  return path.join(dizin, ...parcalar);
}

// ---- Düz TOML alt kümesi: anahtar = "metin" | true|false | 123 ----
function sahipTomlCoz(metin) {
  const sonuc = {};
  String(metin).split(/\r?\n/).forEach((ham, i) => {
    const satir = ham.trim();
    if (satir === '' || satir.startsWith('#')) return;
    const e = /^([a-z0-9_]+)\s*=\s*(.*)$/.exec(satir);
    const hata = () => new VeriHata('GecersizToml', `SAHIP.toml satır ${i + 1} geçersiz: ${ham}`, { satir: i + 1 });
    if (!e) throw hata();
    let d = e[2].replace(/\s+#.*$/, '');
    if (d === 'true' || d === 'false') sonuc[e[1]] = d === 'true';
    else if (/^-?[0-9]+$/.test(d)) sonuc[e[1]] = Number(d);
    else if (/^"(?:[^"\\]|\\["\\])*"\s*$/.test(d)) sonuc[e[1]] = d.trim().slice(1, -1).replace(/\\(["\\])/g, '$1');
    else throw hata();
  });
  return sonuc;
}
function sahipTomlYaz(nesne) {
  return Object.keys(nesne).map((k) => {
    const v = nesne[k];
    if (!/^[a-z0-9_]+$/.test(k)) throw new VeriHata('GecersizToml', `geçersiz anahtar: ${k}`);
    if (typeof v === 'boolean' || (typeof v === 'number' && Number.isInteger(v))) return `${k} = ${v}`;
    if (typeof v === 'string' && !/[\r\n]/.test(v)) return `${k} = "${v.replace(/[\\"]/g, '\\$&')}"`;
    throw new VeriHata('GecersizToml', `desteklenmeyen değer: ${k}`);
  }).join('\n') + '\n';
}

// ---- Dosya yardımcıları (silme yok) ----
function atomikYaz(hedef, veri) {
  fs.mkdirSync(path.dirname(hedef), { recursive: true });
  const gecici = `${hedef}.gecici-${process.pid}-${++sayac}`;
  fs.writeFileSync(gecici, veri);
  fs.renameSync(gecici, hedef);
}
function zamanDamgasi() {
  return new Date().toISOString().replace(/\.\d+Z$/, '').replace(/[-:]/g, '').replace('T', '-');
}
function eskiyeTasi(dizin, yol, ad) {
  const eski = path.join(dizin, 'eski');
  fs.mkdirSync(eski, { recursive: true });
  const taban = path.join(eski, `${ad}-${zamanDamgasi()}`);
  let hedef = taban;
  for (let n = 1; fs.existsSync(hedef); n++) hedef = `${taban}-${n}`;
  fs.renameSync(yol, hedef);
  return hedef;
}
function kayitOku(dizin) {
  const yol = path.join(dizin, 'SAHIP.toml');
  let metin;
  try { metin = fs.readFileSync(yol, 'utf8'); } catch (e) {
    if (e.code === 'ENOENT') return null;
    throw e;
  }
  return sahipTomlCoz(metin);
}

// ---- Sahip ----
function acSahip(secenek) {
  const { alan, program, surum, env } = secenek;
  const gecici = secenek.gecici === true;
  const dizin = alanYolu(alan, env);
  if (typeof program !== 'string' || !ALAN_RE.test(program)) throw new VeriHata('GecersizAd', `geçersiz program adı: ${program}`);
  fs.mkdirSync(dizin, { recursive: true });
  const mevcut = kayitOku(dizin);
  if (mevcut && mevcut.sahip !== program) {
    // Yalnız asıl (geçici olmayan) sahip, geçici sahibi devralabilir.
    if (mevcut.gecici !== true || gecici) throw new BaskaSahip(alan, mevcut.sahip);
    eskiyeTasi(dizin, path.join(dizin, 'SAHIP.toml'), 'SAHIP.toml');
  }
  const kayit = { sema: 1, alan, sahip: program, gecici, yazan_surum: String(surum || ''), son_yazim: new Date().toISOString().replace(/\.\d+Z$/, 'Z') };
  atomikYaz(path.join(dizin, 'SAHIP.toml'), sahipTomlYaz(kayit));
  return {
    alan, dizin, kayit,
    yaz(ad, metin) { atomikYaz(altYol(dizin, ad), metin); },
    oneriler() {
      const od = path.join(dizin, 'oneriler');
      if (!fs.existsSync(od)) return [];
      return fs.readdirSync(od).filter((f) => f.endsWith('.json')).sort().map((f) => {
        try { return { id: f.slice(0, -5), ...JSON.parse(fs.readFileSync(path.join(od, f), 'utf8')) }; } catch (_) { return null; }
      }).filter(Boolean);
    },
    oneriKapat(id) {
      if (!/^[0-9a-z-]+$/.test(String(id))) throw new VeriHata('GecersizAd', `geçersiz öneri kimliği: ${id}`);
      const od = path.join(dizin, 'oneriler');
      fs.mkdirSync(path.join(od, 'eski'), { recursive: true });
      fs.renameSync(path.join(od, `${id}.json`), path.join(od, 'eski', `${id}.json`));
    },
  };
}

// ---- Okuyan: alan dosyası yazamaz; yalnız oneriler/ altına öneri bırakır ----
function acOkur(secenek) {
  const { alan, program, env } = secenek;
  const dizin = alanYolu(alan, env);
  if (typeof program !== 'string' || !ALAN_RE.test(program)) throw new VeriHata('GecersizAd', `geçersiz program adı: ${program}`);
  const kayit = kayitOku(dizin);
  if (!kayit) throw new VeriHata('SahipYok', `'${alan}' alanında SAHIP.toml yok`);
  return {
    alan, dizin, kayit,
    sahip() { return kayit; },
    oku(ad, kodlama) {
      try { return fs.readFileSync(altYol(dizin, ad), kodlama === undefined ? 'utf8' : kodlama); } catch (e) {
        if (e.code === 'ENOENT') return null;
        throw e;
      }
    },
    oner(degisiklik) {
      const metin = JSON.stringify({ sema: 1, oneren: program, zaman: new Date().toISOString(), degisiklik });
      if (Buffer.byteLength(metin) > ONERI_AZAMI) throw new VeriHata('BoyutAsildi', 'öneri 256 KiB sınırını aşıyor');
      const id = `${Date.now()}-${process.pid}-${++sayac}`;
      atomikYaz(path.join(dizin, 'oneriler', `${id}.json`), metin);
      return id;
    },
  };
}

// ---- Göç: kopyala + eskiyi .tasindi yap; bir kez ----
function goc(secenek) {
  const { eskiYol, alan, hedefAd, env } = secenek;
  try {
    if (fs.existsSync(`${eskiYol}.tasindi`)) return { durum: 'zaten' };
    if (!fs.existsSync(eskiYol)) return { durum: 'yok' };
    const hedef = altYol(alanYolu(alan, env), hedefAd);
    if (fs.existsSync(hedef)) return { durum: 'hedef-var' };
    atomikYaz(hedef, fs.readFileSync(eskiYol));
    fs.renameSync(eskiYol, `${eskiYol}.tasindi`);
    return { durum: 'tasindi', hedef };
  } catch (e) {
    if (e instanceof VeriHata) throw e;
    return { durum: 'hata', hata: e.message }; // alan yazılamıyor: eski konumla devam, günlüğe yazılır
  }
}

module.exports = { VeriHata, BaskaSahip, veriKok, alanYolu, acSahip, acOkur, sahipTomlCoz, sahipTomlYaz, goc };
