// Sahip: Yusuf. yazan: [Claude#taa-paket-T2 2026-11-05]
// taa-veri.js v2 — kaynak: yusufbesir/taa-paket/araclar/js/taa-veri.js; programlar kopyalar, elle düzenlemez
'use strict';
// TAA veri alanı kitaplığı (docs/T2-TASARIM.md §1, §1.2; docs/veri-alani.md). Bağımlılıksız, CommonJS. Hiçbir şey silinmez:
// üzerine yazılan/taşınan dosyalar eski/ altına ya da .tasindi adına yeniden adlandırılır.
// v2 (güvenlik sertleştirme): 'wx' + fsync ile atomik yazım, SAHIP.toml ilk yazımı yarışsız (wx), öneri okumada
// lstat/boyut/zarf doğrulaması ve dosya adından gelen kimlik, Windows ayrılmış adları ve büyük/küçük harf duyarsız üst adlar.
const fs = require('fs');
const os = require('os');
const path = require('path');

const ALAN_RE = /^[a-z0-9-]{1,32}$/;
const KIMLIK_RE = /^[0-9a-z][0-9a-z-]{0,127}$/;
const ONERI_AZAMI = 256 * 1024;
const AYRILMIS_UST = ['sahip.toml', 'oneriler', 'eski'];
const AYGIT_RE = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(\..*)?$/i;
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
// Alan içi göreli yol. yazma=true ise ayrılmış üst adlar (SAHIP.toml, oneriler, eski; harfe duyarsız) yasaktır.
// Her parçada: '.', '..', boş, ':' (NTFS ADS), denetim karakteri, sonda nokta/boşluk ve Windows aygıt adları reddedilir.
function altYol(dizin, ad, yazma) {
  const kotu = (n) => new VeriHata('YolDisari', `${n}: ${ad}`);
  if (typeof ad !== 'string' || ad === '' || /[\0-\x1f\x7f]/.test(ad)) throw kotu('geçersiz dosya adı');
  if (path.isAbsolute(ad) || path.win32.isAbsolute(ad) || /^[\\/]/.test(ad) || ad.includes(':')) throw kotu('mutlak yol ya da \':\' reddedildi');
  const parcalar = ad.split(/[\\/]/);
  parcalar.forEach((p, i) => {
    if (p === '' || p === '.' || p === '..') throw kotu('.. ya da boş bölüm reddedildi');
    if (/[. ]$/.test(p)) throw kotu('sonda nokta ya da boşluk reddedildi');
    if (AYGIT_RE.test(p)) throw kotu('Windows aygıt adı reddedildi');
    if (yazma && i === 0 && AYRILMIS_UST.includes(p.toLowerCase())) throw kotu('ayrılmış ad reddedildi');
  });
  return path.join(dizin, ...parcalar);
}

// ---- Düz TOML alt kümesi: anahtar = "metin" | true|false | 123 ----
function tomlKacisCoz(s) {
  return s.replace(/\\(["\\ntr])/g, (_, c) => ({ '"': '"', '\\': '\\', n: '\n', t: '\t', r: '\r' })[c]);
}
function sahipTomlCoz(metin) {
  const sonuc = {};
  String(metin).split(/\r?\n/).forEach((ham, i) => {
    const satir = ham.trim();
    if (satir === '' || satir.startsWith('#')) return;
    const e = /^([A-Za-z0-9_-]+)\s*=\s*(.*)$/.exec(satir);
    const hata = () => new VeriHata('GecersizToml', `SAHIP.toml satır ${i + 1} geçersiz: ${ham}`, { satir: i + 1 });
    if (!e) throw hata();
    const d = e[2];
    const dizgi = /^("(?:[^"\\]|\\.)*")\s*(?:#.*)?$/.exec(d);
    if (dizgi) sonuc[e[1]] = tomlKacisCoz(dizgi[1].slice(1, -1));
    else {
      const sade = d.replace(/\s+#.*$/, '').trim();
      if (sade === 'true' || sade === 'false') sonuc[e[1]] = sade === 'true';
      else if (/^-?[0-9]+$/.test(sade)) sonuc[e[1]] = Number(sade);
      else throw hata();
    }
  });
  return sonuc;
}
const BILINEN_SIRA = ['sema', 'alan', 'sahip', 'gecici', 'yazan_surum', 'son_yazim'];
function sahipTomlYaz(nesne) {
  const anahtarlar = BILINEN_SIRA.filter((k) => k in nesne).concat(Object.keys(nesne).filter((k) => !BILINEN_SIRA.includes(k)).sort());
  return anahtarlar.map((k) => {
    const v = nesne[k];
    if (!/^[A-Za-z0-9_-]+$/.test(k)) throw new VeriHata('GecersizToml', `geçersiz anahtar: ${k}`);
    if (typeof v === 'boolean' || (typeof v === 'number' && Number.isInteger(v))) return `${k} = ${v}`;
    if (typeof v === 'string' && !/[\r\n]/.test(v)) return `${k} = "${v.replace(/[\\"]/g, '\\$&').replace(/\t/g, '\\t')}"`;
    throw new VeriHata('GecersizToml', `desteklenmeyen değer: ${k}`);
  }).join('\n') + '\n';
}

// ---- Dosya yardımcıları (silme yok) ----
// Geçici dosya 'wx' ile açılır (var olan bağ izlenmez), fsync'lenir, sonra yeniden adlandırılır. Hata olursa geçici dosya yerinde kalır.
function atomikYaz(hedef, veri) {
  fs.mkdirSync(path.dirname(hedef), { recursive: true });
  const gecici = `${hedef}.gecici-${process.pid}-${++sayac}`;
  const fd = fs.openSync(gecici, 'wx');
  try { fs.writeFileSync(fd, veri); fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
  fs.renameSync(gecici, hedef);
}
// Hedef yoksa yaratır ('wx'); varsa EEXIST hatası döner (çağıran yeniden okur).
function ilkYaz(hedef, veri) {
  fs.mkdirSync(path.dirname(hedef), { recursive: true });
  const fd = fs.openSync(hedef, 'wx');
  try { fs.writeFileSync(fd, veri); fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
}
function zamanDamgasi() {
  return new Date().toISOString().replace(/\.\d+Z$/, '').replace(/[-:]/g, '').replace('T', '-');
}
// <dizin>/eski/<ad>-<zaman>[-n] (duz=true ise önce <ad>) adını 'wx' ile ayırır (yarışta ezme yok), sonra taşır.
function eskiyeTasi(dizin, yol, ad, duz) {
  const eski = path.join(dizin, 'eski');
  fs.mkdirSync(eski, { recursive: true });
  const taban = path.join(eski, `${ad}-${zamanDamgasi()}`);
  for (let n = duz ? -1 : 0; ; n++) {
    const hedef = n === -1 ? path.join(eski, ad) : n === 0 ? taban : `${taban}-${n}`;
    let fd;
    try { fd = fs.openSync(hedef, 'wx'); } catch (e) { if (e.code === 'EEXIST') continue; throw e; }
    fs.closeSync(fd);
    fs.renameSync(yol, hedef); // yer tutucu ezilir; yalnız bu süreç ayırdı
    return hedef;
  }
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
function simdi() { return new Date().toISOString().replace(/\.\d+Z$/, 'Z'); }

// ---- Sahip ----
function acSahip(secenek) {
  const { alan, program, surum, env } = secenek;
  const gecici = secenek.gecici === true;
  const dizin = alanYolu(alan, env);
  if (typeof program !== 'string' || !ALAN_RE.test(program)) throw new VeriHata('GecersizAd', `geçersiz program adı: ${program}`);
  fs.mkdirSync(dizin, { recursive: true });
  const sahipYolu = path.join(dizin, 'SAHIP.toml');
  let kayit;
  for (let deneme = 0; ; deneme++) {
    const mevcut = kayitOku(dizin);
    const yeni = { sema: 1, alan, sahip: program, gecici, yazan_surum: String(surum || ''), son_yazim: simdi() };
    if (!mevcut) {
      // İlk yazım yarışsız: 'wx' ile yalnız bir süreç kazanır; kaybeden yeniden okur.
      try { ilkYaz(sahipYolu, sahipTomlYaz(yeni)); kayit = yeni; break; } catch (e) {
        if (e.code === 'EEXIST' && deneme < 3) continue;
        throw e;
      }
    }
    if (mevcut.sahip === program) {
      kayit = Object.assign({}, mevcut, yeni); // bilinmeyen anahtarlar korunur
      atomikYaz(sahipYolu, sahipTomlYaz(kayit));
      break;
    }
    // Yalnız asıl (geçici olmayan) sahip, geçici sahibi devralabilir.
    if (mevcut.gecici !== true || gecici) throw new BaskaSahip(alan, mevcut.sahip);
    eskiyeTasi(dizin, sahipYolu, 'SAHIP.toml');
    try { ilkYaz(sahipYolu, sahipTomlYaz(yeni)); } catch (e) { if (e.code === 'EEXIST' && deneme < 3) continue; throw e; }
    kayit = yeni;
    break;
  }
  const od = path.join(dizin, 'oneriler');
  return {
    alan, dizin, kayit,
    yaz(ad, metin) { atomikYaz(altYol(dizin, ad, true), metin); },
    oku(ad, kodlama) {
      try { return fs.readFileSync(altYol(dizin, ad, false), kodlama === undefined ? 'utf8' : kodlama); } catch (e) {
        if (e.code === 'ENOENT') return null;
        throw e;
      }
    },
    // Doğrulanmış öneriler: yalnız düz dosya, ≤256 KiB, kimlik dosya adından, zarf {sema:1, oneren, degisiklik}.
    oneriler() {
      let adlar;
      try { adlar = fs.readdirSync(od); } catch (e) { if (e.code === 'ENOENT') return []; throw e; }
      const sonuc = [];
      adlar.filter((f) => f.endsWith('.json')).sort().forEach((f) => {
        const id = f.slice(0, -5);
        if (!KIMLIK_RE.test(id)) return;
        const yol = path.join(od, f);
        try {
          const st = fs.lstatSync(yol);
          if (!st.isFile() || st.size > ONERI_AZAMI) return;
          const o = JSON.parse(fs.readFileSync(yol, 'utf8'));
          if (!o || typeof o !== 'object' || Array.isArray(o) || o.sema !== 1) return;
          if (typeof o.oneren !== 'string' || !ALAN_RE.test(o.oneren) || !('degisiklik' in o)) return;
          sonuc.push({ sema: 1, oneren: o.oneren, zaman: typeof o.zaman === 'string' ? o.zaman : '', degisiklik: o.degisiklik, id });
        } catch (_) { /* bozuk öneri atlanır; oneriKapat ile eski/ altına alınabilir */ }
      });
      return sonuc;
    },
    oneriKapat(id) {
      if (!KIMLIK_RE.test(String(id))) throw new VeriHata('GecersizAd', `geçersiz öneri kimliği: ${id}`);
      return eskiyeTasi(od, path.join(od, `${id}.json`), `${id}.json`, true);
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
      try { return fs.readFileSync(altYol(dizin, ad, false), kodlama === undefined ? 'utf8' : kodlama); } catch (e) {
        if (e.code === 'ENOENT') return null;
        throw e;
      }
    },
    oner(degisiklik) {
      const metin = JSON.stringify({ sema: 1, oneren: program, zaman: new Date().toISOString(), degisiklik });
      if (Buffer.byteLength(metin) > ONERI_AZAMI) throw new VeriHata('BoyutAsildi', 'öneri 256 KiB sınırını aşıyor');
      const id = `${Date.now()}-${program}-${process.pid}-${++sayac}`;
      atomikYaz(path.join(dizin, 'oneriler', `${id}.json`), metin);
      return id;
    },
  };
}

// ---- Göç: kopyala + eskiyi .tasindi yap; bir kez. `ekler`: aynı anda .tasindi yapılacak yardımcı dosyalar (ör. .bak) ----
function goc(secenek) {
  const { eskiYol, alan, hedefAd, env } = secenek;
  const ekler = Array.isArray(secenek.ekler) ? secenek.ekler : [];
  try {
    if (fs.existsSync(`${eskiYol}.tasindi`)) return { durum: 'zaten' };
    if (!fs.existsSync(eskiYol)) return { durum: 'yok' };
    const hedef = altYol(alanYolu(alan, env), hedefAd, true);
    if (fs.existsSync(hedef)) return { durum: 'hedef-var' };
    atomikYaz(hedef, fs.readFileSync(eskiYol));
    fs.renameSync(eskiYol, `${eskiYol}.tasindi`);
    ekler.forEach((ek) => { try { if (fs.existsSync(ek) && !fs.existsSync(`${ek}.tasindi`)) fs.renameSync(ek, `${ek}.tasindi`); } catch (_) { /* yardımcı dosya; günlük çağıranda */ } });
    return { durum: 'tasindi', hedef };
  } catch (e) {
    if (e instanceof VeriHata) throw e;
    return { durum: 'hata', hata: e.message }; // alan yazılamıyor: eski konumla devam, günlüğe yazılır
  }
}

module.exports = { VeriHata, BaskaSahip, veriKok, alanYolu, altYol, acSahip, acOkur, sahipTomlCoz, sahipTomlYaz, goc, ONERI_AZAMI };
