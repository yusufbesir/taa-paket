// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W0-kabuk 2026-11-27]
// TAA Web kabuğu: giriş, modül yükleyici, hash rotası, çekmeceler, hesap yan paneli, tema,
// çevrimdışı gösterge ("Gönderilmeyen N kayıt") ve çıkış uyarısı.
// Modül sözleşmesi: web/moduller/<ad>/index.js -> {ad, baslik, ac(kok, baglam)};
// baglam = {giden, onbellek, yetki(izin), tema}. Modüller veriye yalnız baglam üstünden erişir.
// Sunucudan/önbellekten gelen her metin textContent ile yazılır; hiçbir metin HTML olarak yorumlanmaz.

import { acDB, OLAY_GIDEN_DEGISTI } from '../cekirdek/taa-cekirdek.js';
import {
  rotaCoz,
  modulListesi,
  yetkiVar,
  girisYanitiCoz,
  dbParcasi,
  profilGecerli,
  gostergeMetni,
  cikisUyarisi,
  YETKI_CEVRIMDISI_GUN,
} from './yardimci.js';

const SUNUCU = location.origin; // uçlar "/v1/..." ile başlar; kabuk ile aynı kökten sunulur
const A_PROFIL = 'taa-web-profil'; // localStorage: {kullanici, sube, roller, izinler, zaman} (anahtar YOK)
const A_OTURUM = 'taa-web-oturum'; // sessionStorage: oturum anahtarı; sekme kapanınca gider
const A_TEMA = 'taa-web-tema';
const GONDERIM_ARALIGI_MS = 60_000;
const LISTE_EN_COK = 50;

const $ = (kimlik) => document.getElementById(kimlik);
const genisEkran = window.matchMedia('(min-width: 900px)');

const durum = {
  profil: null,
  oturum: null,
  db: null,
  moduller: [],
  etkin: null, // {ad, kapat}
  rotaSirasi: 0,
  sayim: { bekliyor: 0, gonderildi: 0, reddedildi: 0 },
  sonDurma: null,
  acikCekmece: null, // {oge, dugme}
  sonDuyuru: '',
  swKaydi: null,
};

// ------------------------------------------------------------ küçük yardımcılar

function oge(etiket, ozellikler = {}, ...cocuklar) {
  const o = document.createElement(etiket);
  for (const [ad, deger] of Object.entries(ozellikler)) {
    if (deger === null || deger === undefined || deger === false) continue;
    if (ad === 'metin') o.textContent = String(deger);
    else o.setAttribute(ad, deger === true ? '' : String(deger));
  }
  for (const c of cocuklar) if (c) o.append(c);
  return o;
}

// `depo`: 'localStorage' | 'sessionStorage'. Depolama kapalıysa erişimin kendisi hata fırlatabilir.
function depoOku(depo, anahtar) {
  try {
    return window[depo].getItem(anahtar);
  } catch {
    return null;
  }
}

function depoYaz(depo, anahtar, deger) {
  try {
    if (deger === null) window[depo].removeItem(anahtar);
    else window[depo].setItem(anahtar, deger);
    return true;
  } catch {
    return false;
  }
}

function saatYaz(iso) {
  const t = new Date(iso);
  return Number.isNaN(t.getTime()) ? '' : t.toLocaleString('tr-TR', { dateStyle: 'short', timeStyle: 'short' });
}

// ------------------------------------------------------------ tema

const tema = {
  ad: 'ilmek',
  get kip() {
    return document.documentElement.getAttribute('data-tema') === 'koyu' ? 'koyu' : 'acik';
  },
  get koyu() {
    return this.kip === 'koyu';
  },
};

function temaUygula(kip, kaydet) {
  const k = kip === 'koyu' ? 'koyu' : 'acik';
  document.documentElement.setAttribute('data-tema', k);
  document.querySelector('meta[name="theme-color"]')?.setAttribute('content', k === 'koyu' ? '#251E21' : '#6A2C5A');
  for (const r of document.querySelectorAll('input[name="tema"]')) r.checked = r.value === k;
  if (kaydet) depoYaz('localStorage', A_TEMA, k);
  window.dispatchEvent(new CustomEvent('taa-tema-degisti', { detail: { ad: tema.ad, kip: k } }));
}

// ------------------------------------------------------------ çekmeceler ve yan panel

function solCekmeceDurumu() {
  // Geniş ekranda sol çekmece sabittir; dar ekranda kapalıyken odak almamalı.
  const sol = $('sol-cekmece');
  const acik = durum.acikCekmece?.oge === sol;
  sol.inert = !genisEkran.matches && !acik;
}

function cekmeceKapat(odakGeriVer = true) {
  const a = durum.acikCekmece;
  if (!a) return;
  durum.acikCekmece = null;
  a.oge.classList.remove('acik');
  a.oge.inert = true;
  a.dugme?.setAttribute('aria-expanded', 'false');
  $('perde').hidden = true;
  solCekmeceDurumu();
  if (odakGeriVer && a.dugme?.isConnected && a.dugme.offsetParent !== null) a.dugme.focus();
}

function cekmeceAc(oge_, dugme) {
  if (durum.acikCekmece?.oge === oge_) {
    cekmeceKapat();
    return;
  }
  cekmeceKapat(false);
  durum.acikCekmece = { oge: oge_, dugme };
  oge_.inert = false;
  oge_.classList.add('acik');
  dugme?.setAttribute('aria-expanded', 'true');
  $('perde').hidden = false;
  solCekmeceDurumu();
  const ilk = oge_.querySelector('button, a[href], input');
  (ilk ?? oge_).focus?.();
}

// ------------------------------------------------------------ gösterge ve giden kutusu

async function sayimYenile() {
  if (!durum.db) return;
  try {
    durum.sayim = await durum.db.giden.say();
  } catch (e) {
    durum.sonDurma = `Yerel kayıtlar okunamadı: ${e.message}`;
  }
  gostergeCiz();
  if (durum.acikCekmece?.oge === $('sag-cekmece')) gidenKutusuCiz();
}

function gostergeCiz() {
  const { metin, tur } = gostergeMetni({
    cevrimici: navigator.onLine,
    bekleyen: durum.sayim.bekliyor,
    reddedilen: durum.sayim.reddedildi,
    oturumVar: Boolean(durum.oturum),
    kalici: durum.db ? durum.db.kalici : true,
  });
  $('gosterge').dataset.tur = tur;
  $('gosterge-metin').textContent = metin;
  if (metin !== durum.sonDuyuru) {
    durum.sonDuyuru = metin;
    $('gosterge-duyuru').textContent = metin; // ekran okuyucu yalnız değişince duyurur
  }
  $('panel-oturum').textContent = durum.oturum
    ? 'Açık'
    : 'Kapalı (kayıt eklenebilir; göndermek için yeniden giriş gerekir)';
  $('yeniden-giris').hidden = Boolean(durum.oturum);
  bantCiz();
}

function kayitSatiri(k, modulBasliklari) {
  const li = oge('li');
  li.append(
    oge('strong', { metin: modulBasliklari.get(k.program) ?? k.program }),
    oge('span', { metin: `${k.uc} · ${saatYaz(k.zaman)}` }),
    oge('span', { class: 'kayit-no', metin: k.no }),
  );
  if (k.ret) li.append(oge('span', { class: 'ret', metin: `Ret (HTTP ${k.ret.kod}): ${k.ret.mesaj}` }));
  return li;
}

async function gidenKutusuCiz() {
  if (!durum.db) return;
  const basliklar = new Map(durum.moduller.map((m) => [m.ad, m.baslik]));
  const [bekleyen, reddedilen] = await Promise.all([
    durum.db.giden.liste('bekliyor'),
    durum.db.giden.liste('reddedildi'),
  ]);
  $('giden-ozet').textContent =
    `Gönderilmeyen ${bekleyen.length} · Reddedilen ${reddedilen.length} · Gönderilen ${durum.sayim.gonderildi}`;
  const durma = $('giden-durma');
  durma.hidden = !(durum.sonDurma && bekleyen.length > 0);
  durma.textContent = durum.sonDurma ?? '';
  $('simdi-gonder').disabled = bekleyen.length === 0;
  for (const [kimlik, liste, bos] of [
    ['giden-bekleyen', bekleyen, 'Gönderilmeyen kayıt yok.'],
    ['giden-reddedilen', reddedilen, 'Reddedilen kayıt yok.'],
  ]) {
    const ul = $(kimlik);
    ul.replaceChildren();
    if (liste.length === 0) ul.append(oge('li', { class: 'bos', metin: bos }));
    for (const k of liste.slice(0, LISTE_EN_COK)) ul.append(kayitSatiri(k, basliklar));
    if (liste.length > LISTE_EN_COK) {
      ul.append(oge('li', { class: 'bos', metin: `… ve ${liste.length - LISTE_EN_COK} kayıt daha` }));
    }
  }
}

// ------------------------------------------------------------ gönderim

/** Bekleyenleri gönderir. Çevrimdışıyken ağa çıkmaz. Hiçbir zaman hata fırlatmaz; özeti döndürür. */
async function gonder() {
  if (!durum.db) return null;
  if (!navigator.onLine) {
    durum.sonDurma = 'Çevrimdışı; bağlantı gelince kendiliğinden gönderilecek.';
    await sayimYenile();
    return null;
  }
  let ozet = null;
  try {
    ozet = await durum.db.giden.gonder({
      sunucu: SUNUCU,
      oturum: durum.oturum ?? '',
      fetch: (adres, secenek) => fetch(adres, secenek),
    });
    durum.sonDurma = ozet.durmaNedeni;
    if (ozet.durmaKodu === 401 && durum.oturum) {
      // Sunucu oturumu tanımıyor: anahtarı bırak, kayıtlar bekliyor kalır.
      durum.oturum = null;
      depoYaz('sessionStorage', A_OTURUM, null);
    }
  } catch (e) {
    durum.sonDurma = `Gönderim başlatılamadı: ${e.message}`;
  }
  await sayimYenile();
  return ozet;
}

let eklemeSayaci = null;
function eklemeSonrasiGonder() {
  clearTimeout(eklemeSayaci);
  eklemeSayaci = setTimeout(() => {
    gonder();
    // Sekme arka plana geçerse tarayıcı (destekliyorsa) kabuğu uyandırsın.
    durum.swKaydi?.sync?.register('taa-giden').catch(() => {});
  }, 400);
}

// ------------------------------------------------------------ bant (oturum / yeni sürüm)

let bekleyenSw = null;

function bantCiz() {
  const bant = $('bant');
  const dugme = $('bant-dugme');
  if (durum.profil && !durum.oturum) {
    $('bant-metin').textContent =
      durum.sayim.bekliyor > 0
        ? `Oturum kapalı. Gönderilmeyen ${durum.sayim.bekliyor} kayıt yeniden giriş yapınca gönderilecek.`
        : 'Oturum kapalı. Kayıt ekleyebilirsiniz; sunucuya göndermek için yeniden giriş yapın.';
    dugme.textContent = 'Giriş yap';
    dugme.dataset.is = 'giris';
    bant.hidden = false;
  } else if (bekleyenSw) {
    $('bant-metin').textContent = 'TAA Web\'in yeni sürümü indirildi.';
    dugme.textContent = 'Yenile';
    dugme.dataset.is = 'yenile';
    bant.hidden = false;
  } else {
    bant.hidden = true;
  }
}

// ------------------------------------------------------------ giriş / çıkış

function girisPenceresi({ zorunlu }) {
  const pencere = $('giris');
  const kullanici = $('giris-kullanici');
  $('giris-hata').hidden = true;
  $('giris-parola').value = '';
  $('giris-kod').value = '';
  $('giris-vazgec').hidden = zorunlu;
  pencere.dataset.zorunlu = zorunlu ? '1' : '';
  if (durum.profil) {
    // Oturum yenileme: aynı kullanıcı, aynı yerel veritabanı.
    kullanici.value = durum.profil.kullanici;
    kullanici.readOnly = true;
    $('giris-aciklama').textContent = 'Oturumunuz kapanmış. Kayıtları göndermek için parolanızı yeniden girin.';
  } else {
    kullanici.readOnly = false;
    $('giris-aciklama').textContent = navigator.onLine
      ? 'Kullanıcı adınız ve parolanızla giriş yapın.'
      : 'Şu an çevrimdışısınız. Bu cihazda ilk giriş için sunucuya bağlantı gerekir.';
  }
  if (!pencere.open) pencere.showModal();
  (kullanici.readOnly ? $('giris-parola') : kullanici).focus();
}

async function girisGonder(olay) {
  olay.preventDefault();
  const hata = $('giris-hata');
  const dugme = $('giris-gonder');
  const kullanici = $('giris-kullanici').value.trim();
  const parola = $('giris-parola').value;
  const kod = $('giris-kod').value.trim();
  const hataYaz = (m) => {
    hata.textContent = m;
    hata.hidden = false;
  };
  if (kullanici === '' || parola === '') {
    hataYaz('Kullanıcı adı ve parola gerekli.');
    return;
  }
  hata.hidden = true;
  dugme.disabled = true;
  dugme.textContent = 'Giriş yapılıyor…';
  try {
    let yanit;
    try {
      yanit = await fetch(`${SUNUCU}/v1/giris`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(kod ? { kullanici, parola, kod } : { kullanici, parola }),
        cache: 'no-store',
        credentials: 'omit',
        redirect: 'error',
      });
    } catch {
      hataYaz('Sunucuya ulaşılamadı. Bağlantınızı denetleyip yeniden deneyin.');
      return;
    }
    if (yanit.status === 401 || yanit.status === 403) {
      hataYaz('Kullanıcı adı, parola ya da doğrulama kodu yanlış.');
      return;
    }
    if (!yanit.ok) {
      hataYaz(`Sunucu girişi kabul etmedi (HTTP ${yanit.status}). Biraz sonra yeniden deneyin.`);
      return;
    }
    let sonuc;
    try {
      sonuc = girisYanitiCoz(await yanit.json(), kullanici);
      dbParcasi(sonuc.profil.sube);
      dbParcasi(sonuc.profil.kullanici);
    } catch (e) {
      hataYaz(`Sunucu yanıtı anlaşılamadı: ${e.message}`);
      return;
    }
    const ayniKisi =
      durum.profil && durum.profil.kullanici === sonuc.profil.kullanici && durum.profil.sube === sonuc.profil.sube;
    if (!depoYaz('localStorage', A_PROFIL, JSON.stringify(sonuc.profil))) {
      hataYaz('Tarayıcı depolaması kapalı; giriş bilgisi saklanamadı. Gizli pencere kullanmayın.');
      return;
    }
    depoYaz('sessionStorage', A_OTURUM, sonuc.oturum);
    $('giris-parola').value = '';
    $('giris').close();
    if (ayniKisi && durum.db) {
      durum.profil = sonuc.profil;
      durum.oturum = sonuc.oturum;
      menuCiz();
      await gonder();
    } else {
      location.reload(); // yeni kullanıcı/şube: her şeyi o kişinin veritabanıyla yeniden kur
    }
  } finally {
    dugme.disabled = false;
    dugme.textContent = 'Giriş yap';
  }
}

async function cikisIste() {
  await sayimYenile();
  const uyari = cikisUyarisi(durum.sayim.bekliyor);
  if (!uyari) {
    cikisYap();
    return;
  }
  $('cikis-metin').textContent = uyari;
  $('cikis-sonuc').hidden = true;
  $('cikis-uyarisi').showModal();
  $('cikis-gonder').focus();
}

async function cikisOncesiGonder() {
  const sonuc = $('cikis-sonuc');
  const dugme = $('cikis-gonder');
  dugme.disabled = true;
  sonuc.hidden = false;
  sonuc.textContent = 'Gönderiliyor…';
  try {
    await gonder();
  } finally {
    dugme.disabled = false;
  }
  if (durum.sayim.bekliyor === 0) {
    $('cikis-uyarisi').close();
    cikisYap();
    return;
  }
  $('cikis-metin').textContent = cikisUyarisi(durum.sayim.bekliyor);
  sonuc.textContent = `Gönderilemedi: ${durum.sonDurma ?? 'bilinmeyen neden'}`;
}

function cikisYap() {
  // Yerel veritabanı SİLİNMEZ (silme yasak); yalnız oturum ve profil bırakılır.
  depoYaz('sessionStorage', A_OTURUM, null);
  depoYaz('localStorage', A_PROFIL, null);
  durum.sayim.bekliyor = 0; // beforeunload uyarısı ikinci kez sormasın; kişi az önce onayladı
  history.replaceState(null, '', location.pathname + location.search);
  location.reload();
}

// ------------------------------------------------------------ modüller ve rota

function baglamOlustur(ad) {
  const { giden, onbellek } = durum.db;
  return Object.freeze({
    giden: Object.freeze({
      ekle: (kayit) => giden.ekle({ ...kayit, program: kayit?.program ?? ad }),
      liste: (d) => giden.liste(d),
      say: () => giden.say(),
      gonder: () => gonder(), // gönderim kabuğun işidir; modül çağırırsa kabuk kendi oturumuyla gönderir
    }),
    onbellek: Object.freeze({
      oku: (anahtar) => onbellek.oku(anahtar),
      okuKayit: (anahtar) => onbellek.okuKayit(anahtar),
      yaz: (anahtar, deger, surum) => onbellek.yaz(anahtar, deger, surum),
    }),
    yetki: (izin) => yetkiVar(durum.profil, izin),
    tema,
    kullanici: Object.freeze({ ad: durum.profil.kullanici, sube: durum.profil.sube }),
  });
}

function gorunurModuller() {
  return durum.moduller.filter((m) => !m.izin || yetkiVar(durum.profil, m.izin));
}

function menuCiz() {
  const p = durum.profil;
  $('hesap-ad').textContent = p.kullanici;
  $('hesap-sube').textContent = `Şube ${p.sube}`;
  $('hesap-rozet').textContent = p.kullanici.slice(0, 1).toLocaleUpperCase('tr-TR');
  $('hesap-dugme').setAttribute('aria-label', `Hesap: ${p.kullanici}, şube ${p.sube}`);
  $('panel-kullanici').textContent = p.kullanici;
  $('panel-sube').textContent = p.sube;
  const etkinAd = rotaCoz(location.hash).ad;
  const ul = $('modul-listesi');
  ul.replaceChildren();
  const ana = oge('a', { href: '#/', metin: 'Ana sayfa' });
  if (!etkinAd) ana.setAttribute('aria-current', 'page');
  ul.append(oge('li', {}, ana));
  for (const m of gorunurModuller()) {
    const a = oge('a', { href: `#/${m.ad}`, metin: m.baslik });
    if (m.ad === etkinAd) a.setAttribute('aria-current', 'page');
    ul.append(oge('li', {}, a));
  }
}

function etkinKapat() {
  const e = durum.etkin;
  durum.etkin = null;
  if (!e?.kapat) return;
  try {
    e.kapat();
  } catch (hata) {
    console.error(`[taa-web] ${e.ad} kapatılırken hata`, hata);
  }
}

function bilgiGoster(baslik, metin) {
  const kutu = oge('div', { class: 'bilgi-kutusu' }, oge('h1', { metin: baslik }), oge('p', { metin }));
  kutu.append(oge('p', {}, oge('a', { href: '#/', metin: 'Ana sayfaya dön' })));
  $('icerik').replaceChildren(kutu);
}

function anaSayfaCiz() {
  const sarmal = oge('div', { class: 'ana-sayfa' }, oge('h1', { metin: 'Programlar' }));
  const liste = gorunurModuller();
  if (liste.length === 0) {
    sarmal.append(oge('p', { class: 'soluk', metin: 'Size açık bir program yok. Yöneticinize başvurun.' }));
  } else {
    const ul = oge('ul', { class: 'karolar' });
    for (const m of liste) {
      const a = oge('a', { class: 'karo', href: `#/${m.ad}` }, oge('strong', { metin: m.baslik }));
      if (m.aciklama) a.append(oge('span', { class: 'soluk', metin: m.aciklama }));
      ul.append(oge('li', {}, a));
    }
    sarmal.append(ul);
  }
  $('icerik').replaceChildren(sarmal);
}

function baslikYaz(baslik) {
  $('sayfa-basligi').textContent = baslik ?? '';
  document.title = baslik ? `${baslik} · TAA Web` : 'TAA Web';
}

async function rotaIsle() {
  const { ad } = rotaCoz(location.hash);
  if (durum.acikCekmece) cekmeceKapat(false);
  if (ad && durum.etkin?.ad === ad) return; // alt rota değişimi modülün kendi işidir (hashchange dinler)
  const sira = (durum.rotaSirasi += 1);
  etkinKapat();
  menuCiz();
  if (!ad) {
    baslikYaz('');
    anaSayfaCiz();
    return;
  }
  const tanim = durum.moduller.find((m) => m.ad === ad);
  if (!tanim) {
    baslikYaz('');
    bilgiGoster('Program bulunamadı', `"${ad}" adında bir program yok.`);
    return;
  }
  baslikYaz(tanim.baslik);
  if (tanim.izin && !yetkiVar(durum.profil, tanim.izin)) {
    bilgiGoster(tanim.baslik, 'Bu programı açma yetkiniz yok. Yöneticinize başvurun.');
    return;
  }
  const kok = oge('div');
  $('icerik').replaceChildren(oge('div', { class: 'modul-kok' }, kok));
  kok.append(oge('p', { class: 'soluk', metin: `${tanim.baslik} açılıyor…` }));
  try {
    const dosya = await import(new URL(`../moduller/${ad}/index.js`, import.meta.url).href);
    const modul = typeof dosya.ac === 'function' ? dosya : dosya.default;
    if (typeof modul?.ac !== 'function') throw new Error('modül "ac(kok, baglam)" dışa aktarmıyor');
    if (sira !== durum.rotaSirasi) return; // yüklenirken kişi başka yere gitti
    kok.replaceChildren();
    durum.etkin = { ad, kapat: null };
    const donen = await modul.ac(kok, baglamOlustur(ad));
    const kapat =
      typeof donen === 'function'
        ? donen
        : typeof donen?.kapat === 'function'
          ? () => donen.kapat()
          : typeof modul.kapat === 'function'
            ? () => modul.kapat(kok)
            : null;
    if (sira !== durum.rotaSirasi) {
      try {
        kapat?.();
      } catch {
        /* zaten ayrıldı */
      }
      return;
    }
    durum.etkin.kapat = kapat;
  } catch (e) {
    console.error(`[taa-web] ${ad} açılamadı`, e);
    if (sira !== durum.rotaSirasi) return;
    durum.etkin = null;
    bilgiGoster(
      tanim.baslik,
      navigator.onLine
        ? 'Bu program açılamadı. Sayfayı yenileyip yeniden deneyin; sürerse yöneticinize bildirin.'
        : 'Bu program bu cihaza henüz indirilmemiş. İnternet ya da şube ağı gelince bir kez açın; sonra çevrimdışı da çalışır.',
    );
  }
  if (sira === durum.rotaSirasi) $('icerik').focus({ preventScroll: true });
}

async function modulleriYukle() {
  try {
    const y = await fetch(new URL('moduller.json', import.meta.url), { cache: 'no-cache' });
    if (!y.ok) throw new Error(`HTTP ${y.status}`);
    return modulListesi(await y.json());
  } catch (e) {
    console.error('[taa-web] moduller.json okunamadı', e);
    return [];
  }
}

// ------------------------------------------------------------ service worker

async function swKur() {
  if (!('serviceWorker' in navigator)) return;
  try {
    const kayit = await navigator.serviceWorker.register('sw.js');
    durum.swKaydi = kayit;
    const izle = (sw) => {
      if (!sw) return;
      const bak = () => {
        if (sw.state === 'installed' && navigator.serviceWorker.controller) {
          bekleyenSw = sw;
          bantCiz();
        }
      };
      sw.addEventListener('statechange', bak);
      bak();
    };
    izle(kayit.waiting);
    kayit.addEventListener('updatefound', () => izle(kayit.installing));
    navigator.serviceWorker.addEventListener('message', (m) => {
      if (m.data?.tur === 'gonder') gonder();
    });
    let yenilendi = false;
    navigator.serviceWorker.addEventListener('controllerchange', () => {
      if (yenilendi || !bekleyenSw) return; // ilk kurulumda sayfayı yenileme
      yenilendi = true;
      location.reload();
    });
  } catch (e) {
    console.error('[taa-web] service worker kurulamadı; çevrimdışı açılış çalışmaz', e);
  }
}

// ------------------------------------------------------------ başlat

function olaylariBagla() {
  $('sol-dugme').addEventListener('click', () => cekmeceAc($('sol-cekmece'), $('sol-dugme')));
  $('gosterge').addEventListener('click', () => {
    cekmeceAc($('sag-cekmece'), $('gosterge'));
    if (durum.acikCekmece) gidenKutusuCiz();
  });
  $('hesap-dugme').addEventListener('click', () => cekmeceAc($('hesap-paneli'), $('hesap-dugme')));
  $('perde').addEventListener('click', () => cekmeceKapat());
  for (const d of document.querySelectorAll('[data-kapat]')) d.addEventListener('click', () => cekmeceKapat());
  document.addEventListener('keydown', (o) => {
    if (o.key === 'Escape' && durum.acikCekmece && !document.querySelector('dialog[open]')) cekmeceKapat();
  });
  genisEkran.addEventListener('change', () => {
    if (durum.acikCekmece?.oge === $('sol-cekmece')) cekmeceKapat(false);
    solCekmeceDurumu();
  });

  for (const r of document.querySelectorAll('input[name="tema"]')) {
    r.addEventListener('change', () => r.checked && temaUygula(r.value, true));
  }

  $('simdi-gonder').addEventListener('click', async () => {
    const d = $('simdi-gonder');
    d.disabled = true;
    d.textContent = 'Gönderiliyor…';
    try {
      await gonder();
    } finally {
      d.textContent = 'Şimdi gönder';
      await gidenKutusuCiz();
    }
  });

  $('bant-dugme').addEventListener('click', () => {
    if ($('bant-dugme').dataset.is === 'yenile') bekleyenSw?.postMessage({ tur: 'atla' });
    else girisPenceresi({ zorunlu: false });
  });
  $('yeniden-giris').addEventListener('click', () => {
    cekmeceKapat(false);
    girisPenceresi({ zorunlu: false });
  });

  $('giris-formu').addEventListener('submit', girisGonder);
  $('giris-vazgec').addEventListener('click', () => $('giris').close());
  $('giris').addEventListener('cancel', (o) => {
    if ($('giris').dataset.zorunlu) o.preventDefault(); // ilk girişte Esc pencereyi kapatmasın
  });

  $('cikis').addEventListener('click', () => {
    cekmeceKapat(false);
    cikisIste();
  });
  $('cikis-vazgec').addEventListener('click', () => $('cikis-uyarisi').close());
  $('cikis-gonder').addEventListener('click', cikisOncesiGonder);
  $('cikis-yine').addEventListener('click', () => {
    $('cikis-uyarisi').close();
    cikisYap();
  });

  window.addEventListener('hashchange', rotaIsle);
  window.addEventListener(OLAY_GIDEN_DEGISTI, (o) => {
    if (o.detail?.db && o.detail.db !== durum.db?.ad) return;
    sayimYenile();
    if (o.detail?.neden === 'eklendi') eklemeSonrasiGonder();
  });
  window.addEventListener('online', () => gonder());
  window.addEventListener('offline', () => gostergeCiz());
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') gonder(); // Safari'de Background Sync yok
  });
  setInterval(() => {
    if (durum.sayim.bekliyor > 0 && document.visibilityState === 'visible') gonder();
  }, GONDERIM_ARALIGI_MS);

  // Sekme kapatılırken / sayfadan çıkılırken bekleyen kayıt varsa tarayıcı uyarısı.
  window.addEventListener('beforeunload', (o) => {
    if (durum.sayim.bekliyor > 0) {
      o.preventDefault();
      o.returnValue = '';
    }
  });
}

function profilOku() {
  try {
    const p = JSON.parse(depoOku('localStorage', A_PROFIL) ?? 'null');
    if (!profilGecerli(p)) return null;
    dbParcasi(p.sube);
    dbParcasi(p.kullanici);
    return p;
  } catch {
    return null;
  }
}

async function baslat() {
  temaUygula(depoOku('localStorage', A_TEMA), false);
  olaylariBagla();
  swKur();

  durum.profil = profilOku();
  if (!durum.profil) {
    // Profil yok ya da 7 günlük çevrimdışı süre dolmuş: giriş şart. Yerel kayıtlar yerinde durur.
    depoYaz('sessionStorage', A_OTURUM, null);
    $('surum-notu').textContent = '';
    girisPenceresi({ zorunlu: true });
    return;
  }
  durum.oturum = depoOku('sessionStorage', A_OTURUM) || null;
  try {
    durum.db = acDB(dbParcasi(durum.profil.sube), dbParcasi(durum.profil.kullanici));
    await durum.db.giden.say(); // veritabanı gerçekten açılıyor mu
  } catch (e) {
    $('kabuk').hidden = false;
    bilgiGoster(
      'Yerel kayıt alanı açılamadı',
      `Bu tarayıcı kayıtları saklayamıyor (${e.message}). Gizli pencereyi kapatıp normal pencerede açın.`,
    );
    return;
  }
  navigator.storage?.persist?.().catch(() => {}); // tarayıcı yerel kayıtları kendiliğinden silmesin
  $('surum-notu').textContent =
    `Çevrimdışı kullanım son girişten sonra ${YETKI_CEVRIMDISI_GUN} gün geçerlidir. Son giriş: ${saatYaz(durum.profil.zaman)}.`;

  durum.moduller = await modulleriYukle();
  $('kabuk').hidden = false;
  solCekmeceDurumu();
  await sayimYenile();
  await rotaIsle();
  gonder();
}

baslat();
