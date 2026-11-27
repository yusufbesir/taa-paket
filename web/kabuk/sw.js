// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W0-kabuk 2026-11-27]
// TAA Web service worker: kabuk + çekirdek + modül dosyalarını önbelleğe alır; sunucuya ulaşılamasa da uygulama açılır.
// Klasik betiktir (modül SW her tarayıcıda yok). Veri uçlarına (/v1/...) HİÇ dokunmaz: kayıtlar çekirdeğin
// giden kutusundan gider, önbellekten yanıtlanmaz.
// Kabuk dosyası eklenince/çıkınca KABUK listesini ve SURUM'u güncelleyin (web/test/kabuk.test.js denetler).

const SURUM = 'taa-web-v1';
const KABUK = [
  './',
  './index.html',
  './kabuk.css',
  './tema.js',
  './uygulama.js',
  './yardimci.js',
  './moduller.json',
  './manifest.webmanifest',
  './simge.svg',
  './simge-192.png',
  './simge-512.png',
  '../cekirdek/taa-cekirdek.js',
];
const WEB_KOKU = new URL('../', self.location.href); // .../web/

function dosyaYoluGecerli(yol) {
  return (
    typeof yol === 'string' &&
    /^moduller\/[A-Za-z0-9._/-]{1,200}$/.test(yol) &&
    !yol.split('/').some((p) => p === '' || p === '.' || p === '..')
  );
}

/** moduller.json içindeki modül dosyaları. Eksik dosya kurulumu düşürmez (modül henüz yazılmamış olabilir). */
async function modulleriOnbellegeAl(onbellek) {
  let liste = [];
  try {
    const y = await fetch('./moduller.json', { cache: 'no-cache' });
    const j = await y.json();
    liste = Array.isArray(j) ? j : Array.isArray(j.moduller) ? j.moduller : [];
  } catch (e) {
    return;
  }
  const yollar = new Set();
  for (const m of liste) {
    if (!m || typeof m.ad !== 'string' || !/^[a-z0-9-]{1,64}$/.test(m.ad)) continue;
    yollar.add(`../moduller/${m.ad}/index.js`);
    for (const d of Array.isArray(m.dosyalar) ? m.dosyalar : []) {
      if (dosyaYoluGecerli(d)) yollar.add(`../${d}`);
    }
  }
  await Promise.allSettled(
    [...yollar].map(async (yol) => {
      const y = await fetch(yol, { cache: 'no-cache' });
      if (y.ok) await onbellek.put(yol, y);
    }),
  );
}

self.addEventListener('install', (olay) => {
  olay.waitUntil(
    (async () => {
      const onbellek = await caches.open(SURUM);
      // Kabuk eksiksiz inmeden kurulum tamam sayılmaz (yarım kabuk çevrimdışı açılmaz).
      await onbellek.addAll(KABUK.map((yol) => new Request(yol, { cache: 'no-cache' })));
      await modulleriOnbellegeAl(onbellek);
    })(),
  );
});

self.addEventListener('activate', (olay) => {
  olay.waitUntil(
    (async () => {
      // Yalnız bu uygulamanın ESKİ dosya önbellekleri bırakılır; kullanıcı verisi (IndexedDB) burada değildir.
      for (const ad of await caches.keys()) {
        if (ad.startsWith('taa-web-') && ad !== SURUM) await caches.delete(ad);
      }
      await self.clients.claim();
    })(),
  );
});

self.addEventListener('message', (olay) => {
  if (olay.data && olay.data.tur === 'atla') self.skipWaiting(); // kişi "Yenile"ye bastı
});

// Background Sync (olan tarayıcılarda): oturum anahtarı sayfadadır, bu yüzden gönderimi açık sayfaya yaptırırız.
self.addEventListener('sync', (olay) => {
  if (olay.tag !== 'taa-giden') return;
  olay.waitUntil(
    (async () => {
      const sayfalar = await self.clients.matchAll({ type: 'window', includeUncontrolled: true });
      for (const s of sayfalar) s.postMessage({ tur: 'gonder' });
    })(),
  );
});

async function agdanAlVeSakla(istek) {
  const yanit = await fetch(istek);
  if (yanit.ok && yanit.type === 'basic') {
    const onbellek = await caches.open(SURUM);
    await onbellek.put(istek, yanit.clone());
  }
  return yanit;
}

self.addEventListener('fetch', (olay) => {
  const istek = olay.request;
  if (istek.method !== 'GET') return;
  const adres = new URL(istek.url);
  if (adres.origin !== self.location.origin) return;
  if (!adres.pathname.startsWith(WEB_KOKU.pathname)) return; // kabuk kökü dışı
  if (/\/v1\//.test(adres.pathname)) return; // veri uçları: her zaman doğrudan ağ
  if (istek.headers.has('range')) return;

  olay.respondWith(
    (async () => {
      const onbellek = await caches.open(SURUM);
      const kayitli = await onbellek.match(istek, { ignoreSearch: true });
      if (kayitli) {
        // Önce önbellek (anında ve çevrimdışı açılır); arka planda tazele, bir sonraki açılışta yeni sürüm gelir.
        olay.waitUntil(agdanAlVeSakla(istek).catch(() => {}));
        return kayitli;
      }
      try {
        return await agdanAlVeSakla(istek);
      } catch (e) {
        if (istek.mode === 'navigate') {
          const kabuk = await onbellek.match('./index.html');
          if (kabuk) return kabuk;
        }
        return new Response('Çevrimdışı: bu dosya bu cihaza henüz indirilmemiş.', {
          status: 503,
          headers: { 'Content-Type': 'text/plain; charset=utf-8' },
        });
      }
    })(),
  );
});
