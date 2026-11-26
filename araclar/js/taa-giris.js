// Sahip: Yusuf. yazan: [Claude#KM2-hesap-giris 2026-11-25]
// taa-giris.js v1 — kaynak: yusufbesir/taa-paket/araclar/js/taa-giris.js; programlar kopyalar, elle düzenlemez.
'use strict';
// TAA ortak giriş köprüsü, Electron ana süreç (Node) tarafı. Plan KM1: "Electron programları için küçük JS köprüsü".
// Ağa kendisi çıkmaz: `taa-kimlik` aracını çalıştırır (cihaz imzası, yedekli yollar, sunucu imzası doğrulaması, Windows
// Kimlik Bilgisi Yöneticisi orada). Şifre yalnız aracın standart girdisine yazılır; hiçbir yere kaydedilmez, günlüğe
// basılmaz, hata iletisine girmez. Oturum anahtarı bu modüle hiç gelmez (araç basmaz).
//
// Kullanım (main.js):
//   const giris = require('./taa/taa-giris');
//   giris.ipcKur(ipcMain, { program: 'tutaa' });
// preload.js:
//   require('./taa/taa-giris').preloadKur(contextBridge, ipcRenderer);   // window.taaGiris
// Ekran (renderer): taa-giris-ekrani.js.
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawn } = require('child_process');

const SURE_MS = 30000;
const CIKTI_EN_COK = 64 * 1024;
const KULLANICI_RE = /^[^\s]{1,64}$/;
const PROGRAM_RE = /^[a-z0-9-]{1,32}$/;

// ---- Aracı bulma: TAA_KIMLIK → <TAA kökü>/kayit.json seçili sürüm → <kök>/depo/taa-kimlik/<sürüm>/bin ----
function taaKok(env) {
  if (env.TAA_KOK) return env.TAA_KOK;
  if (process.platform === 'win32') return 'D:\\Programlar\\TAA';
  return path.join(env.XDG_DATA_HOME || path.join(env.HOME || os.homedir(), '.local', 'share'), 'TAA');
}

// `paketli`: kurulu (paketlenmiş) programda TAA_KIMLIK / TAA_KOK ortam değişkenleri yok sayılır;
// yalnız sabit TAA kökü kullanılır (D:\Programlar\TAA; yazma izni yalnız yöneticide, TAA Kur ayarlar).
function kimlikYolu(env, paketli) {
  env = env || process.env;
  if (paketli) env = { HOME: env.HOME, XDG_DATA_HOME: env.XDG_DATA_HOME };
  const exe = process.platform === 'win32' ? 'taa-kimlik.exe' : 'taa-kimlik';
  if (env.TAA_KIMLIK) return fs.existsSync(env.TAA_KIMLIK) ? env.TAA_KIMLIK : null;
  const kok = taaKok(env);
  try {
    const kayit = JSON.parse(fs.readFileSync(path.join(kok, 'kayit.json'), 'utf8'));
    const secili = kayit && kayit.paketler && kayit.paketler['taa-kimlik'] && kayit.paketler['taa-kimlik'].secili;
    // Sürüm adı rakamla başlar, '..' içermez; çözülen yol <kök>/depo/taa-kimlik altında kalmalı.
    if (typeof secili === 'string' && /^[0-9][0-9a-z.+-]{0,31}$/.test(secili) && !secili.includes('..')) {
      const taban = path.resolve(kok, 'depo', 'taa-kimlik');
      const y = path.resolve(taban, secili, 'bin', exe);
      if (y.startsWith(taban + path.sep) && fs.existsSync(y)) return y;
    }
  } catch (_) { /* kayıt yok ya da bozuk: aşağıdaki yedek */ }
  const yanimda = path.join(kok, 'bin', exe);
  return fs.existsSync(yanimda) ? yanimda : null;
}

// ---- Aracı çalıştırma ----
function jsonSatir(metin) {
  const satirlar = String(metin).split(/\r?\n/).map((s) => s.trim()).filter(Boolean);
  for (let i = satirlar.length - 1; i >= 0; i--) {
    if (satirlar[i].startsWith('{')) {
      try { return JSON.parse(satirlar[i]); } catch (_) { /* sonraki */ }
    }
  }
  return null;
}

// Promise<{kod, cikti, hata}>; asla fırlatmaz. `girdi` (şifre) yalnız stdin'e yazılır.
function calistir(args, girdi, secenek) {
  const s = secenek || {};
  const env = s.env || process.env;
  const exe = kimlikYolu(env, s.paketli === true);
  if (!exe) return Promise.resolve({ kod: -1, cikti: null, hata: null });
  return new Promise((coz) => {
    let cikti = '';
    let hata = '';
    let bitti = false;
    const son = (kod) => { if (!bitti) { bitti = true; coz({ kod, cikti: jsonSatir(cikti), hata: jsonSatir(hata) }); } };
    let p;
    try {
      p = spawn(exe, [...args, '--json'], { env, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] });
    } catch (_) { son(-1); return; }
    const zaman = setTimeout(() => { try { p.kill(); } catch (_) { /* yok */ } son(3); }, s.sureMs || SURE_MS);
    p.stdout.on('data', (b) => { if (cikti.length < CIKTI_EN_COK) cikti += b; });
    p.stderr.on('data', (b) => { if (hata.length < CIKTI_EN_COK) hata += b; });
    p.on('error', () => { clearTimeout(zaman); son(-1); });
    p.on('close', (kod) => { clearTimeout(zaman); son(kod == null ? 1 : kod); });
    p.stdin.on('error', () => { /* araç girdiyi okumadan çıktı */ });
    p.stdin.end(girdi == null ? '' : String(girdi).replace(/[\r\n]/g, '') + '\n');
  });
}

// ---- Sonuçları kişiye uygun duruma çevirme ----
const ILETI = {
  arac_yok: 'Giriş aracı (taa-kimlik) bu bilgisayarda kurulu değil. TAA Kur ile kurun ya da yöneticinize haber verin.',
  baglanti: 'Sunucuya ulaşılamadı. İnternet ya da ağ bağlantısını denetleyip yeniden deneyin.',
  hatali: 'Kullanıcı adı ya da şifre hatalı.',
  kilitli: 'Hesabınız kilitlendi. "Yöneticiye başvur" ile yöneticiden kilidi açmasını isteyin.',
  sinir: 'Bu bilgisayardan çok fazla hatalı deneme yapıldı. Bir saat sonra yeniden deneyin.',
  mesgul: 'Bu bilgisayardan önceki istek sürüyor. Birazdan yeniden deneyin.',
  onay_bekliyor: 'Hesabınız yönetici onayı bekliyor. Yönetici onaylayınca yeniden deneyin.',
  cihaz: 'Bu bilgisayar sunucuya kayıtlı ya da onaylı değil. Yöneticiden bilgisayarı onaylamasını isteyin.',
  program_kapali: 'Bu program hesabınıza ya da şubenize açık değil. Yöneticinize başvurun.',
  sube_yetkisiz: 'Bu bilgisayarın şubesinde yetkiniz yok. Kendi şubenizin bilgisayarından girin ya da yöneticinize başvurun.',
  sube_cihazi: 'Başvuru yalnız mağaza bilgisayarından yapılır.',
  girdi: 'Kullanıcı adını ve şifreyi doğru yazın.',
  oturum_yok: 'Oturum yok. Giriş yapın.',
  bilinmiyor: 'Giriş yapılamadı. Yeniden deneyin; sürerse yöneticinize haber verin.',
};

// Rust iletisinin içine gömülü sunucu gövdesi ({"hata": kod, "ileti": "..."}) varsa çözülür.
function sunucuGovdesi(m) {
  const i = m.indexOf('{');
  const j = m.lastIndexOf('}');
  if (i < 0 || j <= i) return null;
  try { const v = JSON.parse(m.slice(i, j + 1)); return v && typeof v === 'object' ? v : null; } catch (_) { return null; }
}

function durumBul(r) {
  if (r.kod === -1) return 'arac_yok';
  const h = r.hata || {};
  const http = Number(h.http) || 0;
  const m = String(h.hata || '');
  const g = sunucuGovdesi(m) || {};
  const kod = String(g.hata || '');
  if (http === 423) return 'kilitli';
  if (http === 429) return kod === 'cihaz_mesgul' || m.includes('cihaz_mesgul') || m.includes('önceki istek') ? 'mesgul' : 'sinir';
  if (http === 403) {
    if (kod === 'hesap_onay_bekliyor' || m.includes('onayı bekliyor')) return 'onay_bekliyor';
    if (kod === 'sube_yetkisiz') return 'sube_yetkisiz';
    if (kod === 'sube_cihazi_gerekli') return 'sube_cihazi';
    return 'cihaz';
  }
  if (http === 401) return kod.startsWith('cihaz') ? 'cihaz' : 'hatali';
  if (r.kod === 3) return 'baglanti';
  if (r.kod === 2) return 'girdi';
  if (r.kod === 5) return m.includes('cihaz') ? 'cihaz' : 'oturum_yok';
  return 'bilinmiyor';
}

const red = (durum) => ({ tamam: false, durum, ileti: ILETI[durum] || ILETI.bilinmiyor });

// Sunucuya giden adres TAA_SUNUCU ya da <veri kökü>/yol listesi; program `sunucu` vermezse araç kendisi seçer.
const sunucuArg = (s) => (s && s.sunucu ? ['--sunucu', String(s.sunucu)] : []);

// ---- Komutlar ----
async function giris(istek, secenek) {
  const s = secenek || {};
  const kullanici = String((istek && istek.kullanici) || '').trim();
  const parola = String((istek && istek.parola) || '');
  if (!KULLANICI_RE.test(kullanici) || !parola || /[\r\n]/.test(parola)) return red('girdi');
  const r = await calistir(['giris', '--kullanici', kullanici, ...sunucuArg(s)], parola, s);
  if (r.kod !== 0 || !r.cikti) return red(durumBul(r));
  const o = r.cikti;
  const oturum = {
    kullanici: o.kullanici, sube_id: o.sube_id == null ? null : o.sube_id,
    roller: Array.isArray(o.roller) ? o.roller : [], izinler: Array.isArray(o.izinler) ? o.izinler : [],
    programlar: Array.isArray(o.programlar) ? o.programlar : [], uyari_sayisi: Number(o.uyari_sayisi) || 0,
  };
  // Program bu kişiye/şubeye kapalıysa açılmaz. Boş liste = hiçbir program açık değil (ret).
  // Çıkış yapılmaz: ortak oturum başka programlarda sürer, yalnız bu program reddedilir.
  if (!programAcik(oturum, s.program)) return red('program_kapali');
  kilitSil(s); // başarılı giriş (kilit açma dahil) bu programın kilit işaretini kaldırır
  return { tamam: true, oturum };
}

async function basvur(istek, secenek) {
  const s = secenek || {};
  const kullanici = String((istek && istek.kullanici) || '').trim();
  const yeni = String((istek && istek.yeniParola) || '');
  if (!KULLANICI_RE.test(kullanici) || /[\r\n]/.test(yeni)) return red('girdi');
  const r = await calistir(['basvur', '--kullanici', kullanici, ...sunucuArg(s)], yeni, s);
  const sayi = r.cikti && r.cikti.dogrulama_sayisi;
  if (r.kod !== 0 || typeof sayi !== 'string' || !/^[0-9]{2}$/.test(sayi)) {
    // Sunucunun şifre kuralı iletisi (uzunluk, yaygın şifre…) kişiye aynen gösterilir; şifrenin kendisi iletide yoktur.
    const g = r.hata && Number(r.hata.http) === 400 ? sunucuGovdesi(String(r.hata.hata || '')) : null;
    if (g && g.hata === 'parola' && typeof g.ileti === 'string') return { tamam: false, durum: 'parola', ileti: g.ileti };
    return red(durumBul(r));
  }
  return {
    tamam: true, dogrulama_sayisi: sayi, tur: yeni ? 'sifre' : 'kilit',
    ileti: 'Başvurunuz yöneticiye iletildi. Yöneticiye bu iki haneli sayıyı söyleyin; onaylanınca giriş yapabilirsiniz.',
  };
}

// Süren oturum. Program verilmişse ve bu programa kapalıysa null (giriş ekranı açılır).
async function durum(secenek) {
  const s = secenek || {};
  const r = await calistir(['durum', ...sunucuArg(s)], null, s);
  if (r.kod !== 0 || !r.cikti) return null;
  const o = r.cikti;
  const oturum = {
    kullanici: o.kullanici, sube_id: o.sube_id == null ? null : o.sube_id,
    roller: Array.isArray(o.roller) ? o.roller : [], izinler: Array.isArray(o.izinler) ? o.izinler : [],
    programlar: Array.isArray(o.programlar) ? o.programlar : [],
  };
  return programAcik(oturum, s.program) ? oturum : null;
}

// Program adı yoksa (ör. Çatı'nın genel denetimi) denetim yapılmaz; varsa listede olmalı. Liste yoksa ya da boşsa kapalı.
function programAcik(oturum, program) {
  if (!program) return true;
  return Array.isArray(oturum.programlar) && oturum.programlar.includes(program);
}

// Çevrimdışı izin kararı (imzalı yetki belgesi). Varsayılan: reddet.
async function yetki(istek, secenek) {
  const s = secenek || {};
  const args = ['yetki', ...sunucuArg(s)];
  // Program ana süreçte sabitlenmişse (s.program) renderer'dan gelen değer yok sayılır.
  const program = s.program || (istek && istek.program);
  const izin = istek && istek.izin;
  if (program) { if (!PROGRAM_RE.test(program)) return { izinli: false }; args.push('--program', program); }
  if (izin) { if (!/^[a-z0-9-]{1,32}\.[a-z0-9_.-]{1,64}$/.test(izin)) return { izinli: false }; args.push('--izin', izin); }
  const r = await calistir(args, null, s);
  return { izinli: r.kod === 0, neden: (r.cikti && r.cikti.neden) || [] };
}

async function cikis(secenek) {
  const s = secenek || {};
  const r = await calistir(['cikis', ...sunucuArg(s)], null, s);
  kilitSil(s);
  return { tamam: r.kod === 0 };
}

// ---- Ekran kilidi (Yusuf 11-26: "Kilit yayını durdurmasın") ----
// Kilit oturumu kapatmaz (sunucudaki oturum ve ona bağlı yayın belirteçleri sürer); yalnız bu programın arayüzü kilitlenir.
// Program kapatılıp açılınca kilit atlanmasın diye işaret programın veri klasörüne yazılır; baslangic işaret varken
// oturumu vermez (giriş ekranı kullanıcı adı dolu, kilitli açılır). İşareti başarılı giriş ya da çıkış kaldırır.
// Kullanıcı adı renderer'dan alınmaz: süren oturumdan okunur.
function kilitDosyasi(s) {
  if (s && s.kilitDosyasi) return String(s.kilitDosyasi);
  let d = s && s.veriKlasoru;
  if (!d) { try { d = require('electron').app.getPath('userData'); } catch (_) { return null; } }
  return d ? path.join(String(d), 'taa-giris-kilit.json') : null;
}
function kilitOku(s) {
  const y = kilitDosyasi(s);
  if (!y) return null;
  try {
    const v = JSON.parse(fs.readFileSync(y, 'utf8'));
    return v && typeof v.kullanici === 'string' && KULLANICI_RE.test(v.kullanici) ? v : null;
  } catch (_) { return null; }
}
function kilitSil(s) {
  const y = kilitDosyasi(s);
  if (y) { try { fs.unlinkSync(y); } catch (_) { /* yoksa sorun değil */ } }
}
async function kilitle(secenek) {
  const s = secenek || {};
  const oturum = await durum(s);
  if (!oturum || !KULLANICI_RE.test(String(oturum.kullanici || ''))) return { tamam: false };
  const y = kilitDosyasi(s);
  if (!y) return { tamam: false, kullanici: oturum.kullanici };
  try {
    fs.mkdirSync(path.dirname(y), { recursive: true });
    fs.writeFileSync(y, JSON.stringify({ kullanici: oturum.kullanici, zaman: Date.now() }), { mode: 0o600 });
  } catch (_) { return { tamam: false, kullanici: oturum.kullanici }; }
  return { tamam: true, kullanici: oturum.kullanici };
}

// ---- Electron bağlantısı ----
const KANAL = { baslangic: 'taa-giris:baslangic', giris: 'taa-giris:giris', basvur: 'taa-giris:basvur', durum: 'taa-giris:durum', yetki: 'taa-giris:yetki', cikis: 'taa-giris:cikis', kilitle: 'taa-giris:kilitle' };

// Açılış kararı: araç kurulu mu, bu programa açık süren oturum var mı. Araç yoksa program eski davranışını sürdürür.
async function baslangic(secenek) {
  const s = secenek || {};
  if (!kimlikYolu(s.env || process.env, s.paketli === true)) return { arac: false, oturum: null };
  const oturum = await durum(s);
  const k = kilitOku(s);
  if (oturum && k && k.kullanici === oturum.kullanici) return { arac: true, oturum: null, kilitli: { kullanici: k.kullanici } };
  if (k) kilitSil(s); // oturum yok ya da başka kullanıcı: eski işaret geçersiz
  return { arac: true, oturum };
}

// Varsayılan gönderen denetimi: yalnız pencerenin kendi ana çerçevesi ve yerel (file:) sayfa.
function gonderenVarsayilan(olay) {
  const f = olay && olay.senderFrame;
  const url = f && typeof f.url === 'string' ? f.url : '';
  if (!f || !olay.sender || f !== olay.sender.mainFrame || !url.startsWith('file:')) throw new Error('taa-giris: yetkisiz gönderen');
}

// secenek: { program, paketli, sunucu, denetle(olay), veriKlasoru?, kilitDosyasi? } — denetle fırlatırsa çağrı reddedilir.
function ipcKur(ipcMain, secenek) {
  const s = Object.assign({}, secenek);
  if (s.program && !PROGRAM_RE.test(s.program)) throw new Error('taa-giris: program adı geçersiz');
  const denetle = typeof s.denetle === 'function' ? s.denetle : gonderenVarsayilan;
  const kur = (kanal, is) => ipcMain.handle(kanal, (olay, i) => { denetle(olay); return is(i && typeof i === 'object' ? i : {}); });
  kur(KANAL.baslangic, () => baslangic(s));
  kur(KANAL.giris, (i) => giris(i, s));
  kur(KANAL.basvur, (i) => basvur(i, s));
  kur(KANAL.durum, () => durum(s));
  kur(KANAL.yetki, (i) => yetki(i, s));
  kur(KANAL.cikis, () => cikis(s));
  kur(KANAL.kilitle, () => kilitle(s));
}

function preloadKur(contextBridge, ipcRenderer) {
  contextBridge.exposeInMainWorld('taaGiris', {
    giris: (kullanici, parola) => ipcRenderer.invoke(KANAL.giris, { kullanici, parola }),
    basvur: (kullanici, yeniParola) => ipcRenderer.invoke(KANAL.basvur, { kullanici, yeniParola }),
    baslangic: () => ipcRenderer.invoke(KANAL.baslangic),
    durum: () => ipcRenderer.invoke(KANAL.durum),
    yetki: (izin) => ipcRenderer.invoke(KANAL.yetki, { izin }),
    cikis: () => ipcRenderer.invoke(KANAL.cikis),
    kilitle: () => ipcRenderer.invoke(KANAL.kilitle),
  });
}

module.exports = { kimlikYolu, calistir, baslangic, giris, basvur, durum, yetki, cikis, kilitle, ipcKur, preloadKur, KANAL, ILETI, _durumBul: durumBul };
