// Sahip: Yusuf. yazan: [Claude Sonnet 5.5 #taa-web-cihaz 2026-11-27]
// Tarayıcı cihaz kimliği (docs/S2-CIHAZ.md ile aynı sözleşme): Ed25519 anahtar çifti WebCrypto'da ÇIKARILAMAZ üretilir,
// IndexedDB'de CryptoKey olarak saklanır (özel anahtar JS'e hiç açılmaz). Her /v1 isteği
// X-TAA-Cihaz (genel anahtarın ilk 8 baytı, 16 hex), X-TAA-Zaman (unix sn), X-TAA-Imza (128 hex) taşır;
// imzalanan bayt dizisi "{YONTEM}\n{YOL}\n{ZAMAN}\n" + gövde baytları (taa_cihaz::imza_mesaji ile aynı).
// Kayıt: POST /v1/cihaz/kayit {acik_anahtar:"ed25519:<hex>", ad[, sube_kodu]} (imzasız tek uç); yönetici Çatı'dan onaylar.
// Ed25519 WebCrypto'da yoksa (eski tarayıcı) fallback YOKTUR: `destekVar()` false döner, kabuk bunu açıkça söyler.

export const KAYIT_UCU = '/v1/cihaz/kayit';
export const DURUM_UCU = '/v1/cihaz/durum';
export const BASLIK_CIHAZ = 'X-TAA-Cihaz';
export const BASLIK_ZAMAN = 'X-TAA-Zaman';
export const BASLIK_IMZA = 'X-TAA-Imza';
const DB_AD = 'taa-cihaz';
const DEPO = 'anahtar';
const KAYIT_ANAHTARI = 'tarayici';
const AD_EN_COK = 64;

const hex = (bayt) => [...new Uint8Array(bayt)].map((b) => b.toString(16).padStart(2, '0')).join('');
const utf8 = (m) => new TextEncoder().encode(m);

export class CihazHata extends Error {
  constructor(kod, mesaj) {
    super(mesaj);
    this.kod = kod;
  }
}

/** Bellek içi depo (Node testi / IndexedDB'siz ortam). Gerçek depo `idbDepo()`. */
export class BellekDepo {
  constructor() {
    this.m = new Map();
  }
  async oku(k) {
    return this.m.get(k) ?? null;
  }
  async yaz(k, v) {
    this.m.set(k, v);
  }
}

export function idbDepo(idb = globalThis.indexedDB) {
  if (!idb) throw new CihazHata('depo-yok', 'Bu tarayıcıda IndexedDB yok; cihaz anahtarı saklanamaz.');
  const ac = () =>
    new Promise((coz, red) => {
      const i = idb.open(DB_AD, 1);
      i.onupgradeneeded = () => i.result.createObjectStore(DEPO);
      i.onsuccess = () => coz(i.result);
      i.onerror = () => red(new CihazHata('depo', i.error?.message ?? 'IndexedDB açılamadı'));
    });
  const is = async (mod, f) => {
    const db = await ac();
    try {
      return await new Promise((coz, red) => {
        const t = db.transaction(DEPO, mod);
        const r = f(t.objectStore(DEPO));
        t.oncomplete = () => coz(r.result);
        t.onerror = () => red(new CihazHata('depo', t.error?.message ?? 'IndexedDB hatası'));
      });
    } finally {
      db.close();
    }
  };
  return {
    oku: (k) => is('readonly', (s) => s.get(k)).then((v) => v ?? null),
    yaz: (k, v) => is('readwrite', (s) => s.put(v, k)).then(() => undefined),
  };
}

/** Ed25519 bu tarayıcıda çalışıyor mu (üretip imzalayarak dener). */
export async function destekVar(subtle = globalThis.crypto?.subtle) {
  if (!subtle) return false;
  try {
    const c = await subtle.generateKey({ name: 'Ed25519' }, false, ['sign', 'verify']);
    await subtle.sign({ name: 'Ed25519' }, c.privateKey, new Uint8Array([1]));
    return true;
  } catch {
    return false;
  }
}

export function adGecerli(ad) {
  return typeof ad === 'string' && ad.trim().length > 0 && ad.trim().length <= AD_EN_COK && /^[\p{L}\p{N}\-_. ]+$/u.test(ad.trim());
}

/** İmzalanan bayt dizisi (taa_cihaz::imza_mesaji). */
export function imzaMesaji(yontem, yol, zaman, govde = new Uint8Array()) {
  const baslik = utf8(`${String(yontem).toUpperCase()}\n${yol}\n${zaman}\n`);
  const g = typeof govde === 'string' ? utf8(govde) : govde;
  const m = new Uint8Array(baslik.length + g.length);
  m.set(baslik, 0);
  m.set(g, baslik.length);
  return m;
}

export class TarayiciCihaz {
  constructor({ depo, subtle = globalThis.crypto?.subtle, saat = () => Date.now() } = {}) {
    this.depo = depo;
    this.subtle = subtle;
    this.saat = saat;
    this.kayit = null; // {ad, kimlik, acik, ciftler, olusturma, sunucu?, durum}
  }

  /** Varsa yükler; yoksa null. */
  async yukle() {
    this.kayit = await this.depo.oku(KAYIT_ANAHTARI);
    return this.kayit;
  }

  /** Anahtar yoksa üretir (özel anahtar çıkarılamaz); varsa DOKUNMAZ. */
  async uret(ad) {
    const var_ = await this.yukle();
    if (var_) return var_;
    if (!adGecerli(ad)) throw new CihazHata('ad', 'Cihaz adı 1-64 karakter (harf, rakam, - _ . boşluk) olmalı.');
    if (!(await destekVar(this.subtle))) {
      throw new CihazHata('destek-yok', 'Bu tarayıcı Ed25519 desteklemiyor; güncel Chrome, Edge, Firefox ya da Safari 17+ gerekir.');
    }
    const ciftler = await this.subtle.generateKey({ name: 'Ed25519' }, false, ['sign', 'verify']);
    const ham = new Uint8Array(await this.subtle.exportKey('raw', ciftler.publicKey));
    const acik = hex(ham);
    const k = { ad: ad.trim(), kimlik: acik.slice(0, 16), acik, ciftler, olusturma: Math.floor(this.saat() / 1000), durum: 'kayitsiz' };
    await this.depo.yaz(KAYIT_ANAHTARI, k);
    this.kayit = k;
    return k;
  }

  gerek() {
    if (!this.kayit) throw new CihazHata('anahtar-yok', 'Cihaz anahtarı henüz üretilmedi.');
    return this.kayit;
  }

  async imzala(yontem, yol, zaman, govde) {
    const k = this.gerek();
    const imza = await this.subtle.sign({ name: 'Ed25519' }, k.ciftler.privateKey, imzaMesaji(yontem, yol, zaman, govde));
    return hex(imza);
  }

  async basliklar(yontem, yol, govde) {
    const zaman = Math.floor(this.saat() / 1000);
    return {
      [BASLIK_CIHAZ]: this.gerek().kimlik,
      [BASLIK_ZAMAN]: String(zaman),
      [BASLIK_IMZA]: await this.imzala(yontem, yol, zaman, govde),
    };
  }

  /** POST /v1/cihaz/kayit (imzasız, idempotent). Dönen: {kimlik, durum}. */
  async kaydet(sunucu, getir, subeKodu) {
    const k = this.gerek();
    const govde = { acik_anahtar: `ed25519:${k.acik}`, ad: k.ad };
    if (subeKodu && String(subeKodu).trim()) govde.sube_kodu = String(subeKodu).trim();
    const y = await getir(`${sunucu}${KAYIT_UCU}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(govde),
      cache: 'no-store',
      credentials: 'omit',
      redirect: 'error',
    });
    if (!(y.status >= 200 && y.status <= 299)) throw new CihazHata('kayit-red', `Sunucu cihaz kaydını kabul etmedi (HTTP ${y.status}).`);
    let g = {};
    try {
      g = JSON.parse(await y.text());
    } catch {
      /* gövde yok */
    }
    const durum = typeof g.durum === 'string' ? g.durum : 'bekliyor';
    this.kayit = { ...k, sunucu: sunucu, durum };
    await this.depo.yaz(KAYIT_ANAHTARI, this.kayit);
    return { kimlik: k.kimlik, durum };
  }

  /**
   * fetch sarmalayıcı: yalnız `sunucu` kökündeki /v1 isteklerini imzalar (kayıt ucu imzasız kalır).
   * Gövde metin olmalı (JSON); imza, gönderilen baytların aynısı üzerindendir.
   */
  imzaliFetch(getir, sunucu) {
    const kok = new URL(sunucu).origin;
    return async (adres, secenek = {}) => {
      const u = new URL(adres, sunucu);
      if (this.kayit && u.origin === kok && u.pathname.startsWith('/v1/') && u.pathname !== KAYIT_UCU) {
        const govde = secenek.body ?? '';
        if (typeof govde !== 'string' && !(govde instanceof Uint8Array)) {
          throw new CihazHata('govde', 'İmzalı istek gövdesi metin ya da bayt dizisi olmalı.');
        }
        const yontem = secenek.method ?? 'GET';
        const baslik = await this.basliklar(yontem, u.pathname + u.search, govde);
        return getir(adres, { ...secenek, headers: { ...(secenek.headers ?? {}), ...baslik } });
      }
      return getir(adres, secenek);
    };
  }
}
