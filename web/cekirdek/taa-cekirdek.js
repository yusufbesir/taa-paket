// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W0-cekirdek 2026-11-27]
// TAA Web çekirdeği: çevrimdışı önce veri katmanı (giden kutusu + önbellek).
// Kaynak: PLAN-TAA-WEB-2026-11-27 §2 katman 2; docs/T6-KUYRUK.md ile aynı kurallar (taa-kuyruk'un tarayıcı eşi).
// Kurallar: yalnız ekleme, silme yok (yalnız durum değişir); aynı `no` ikinci kez eklenmez;
// gönderim sırayla yapılır, sunucuya ulaşılamazsa o kayıtta durur ve sıra bozulmaz.
// Bağımlılık yok. IndexedDB yoksa (Node testi) bellek içi yedek sürücü kullanılır.
// [Claude Fable 5.2 #taa-web-W2-yol 2026-11-27] Eklendi: gönderim yolları (./yol.js; merkez → şube → yerel) ve
// cihaz günlüğü (`Gunluk`: yalnız ekleme, silme yok). `gonder({sunucu, oturum})` eskisi gibi tek yola gönderir.

import { yolluGonder } from './yol.js';

export const OLAY_GIDEN_DEGISTI = 'taa-giden-degisti';
export const DURUMLAR = Object.freeze(['bekliyor', 'gonderildi', 'reddedildi']);
export const NO_EN_COK = 64;
export const GOVDE_EN_COK = 1024 * 1024; // taa-kuyruk ile aynı sınır (bayt)
export const ZAMAN_ASIMI_MS = 60_000; // taa-kuyruk: curl --max-time 60

/** Olaylar burada da yayınlanır (Node'da pencere yoktur). Tarayıcıda ayrıca `window` üstünde yayınlanır. */
export const olaylar = new EventTarget();

const DEPOLAR = ['giden', 'onbellek', 'meta', 'gunluk'];
const ANAHTAR_ALANI = { giden: 'no', onbellek: 'anahtar', meta: 'anahtar', gunluk: 'sira' };
const VT_SURUMU = 2; // 1: giden, onbellek, meta; 2: + gunluk (eksik depo yükseltmede eklenir, hiçbir depo silinmez)
export const GUNLUK_AYRINTI_EN_COK = 4096; // bir günlük satırının ayrıntısı (bayt)

export class TaaHata extends Error {
  constructor(kod, mesaj) {
    super(mesaj);
    this.name = 'TaaHata';
    this.kod = kod;
  }
}

// ---------------------------------------------------------------- doğrulama

export function noGecerli(no) {
  return (
    typeof no === 'string' &&
    no.length >= 1 &&
    no.length <= NO_EN_COK &&
    /^[A-Za-z0-9._-]+$/.test(no) &&
    no !== '.' &&
    no !== '..'
  );
}

/** Uç: `/v1/...`, yalnız `[A-Za-z0-9/_-]`, `..` ve `//` yok (taa-kuyruk `uc_gecerli` + `/v1/` şartı). */
export function ucGecerli(uc) {
  return (
    typeof uc === 'string' &&
    uc.startsWith('/v1/') &&
    uc.length > 4 &&
    uc.length <= 256 &&
    !uc.includes('..') &&
    !uc.includes('//') &&
    /^[A-Za-z0-9/_-]+$/.test(uc)
  );
}

export function programGecerli(program) {
  return typeof program === 'string' && /^[a-z0-9-]{1,64}$/.test(program);
}

// Veritabanı adı "taa-<sube>-<kullanici>" olduğu için parçalarda tire YOKTUR:
// yoksa ("a-b","c") ile ("a","b-c") aynı ada düşer ve şubeler karışırdı.
function parcaGecerli(p) {
  return typeof p === 'string' && /^[A-Za-z0-9._]{1,48}$/.test(p) && p !== '.' && p !== '..';
}

function kopya(deger) {
  return deger === undefined ? undefined : JSON.parse(JSON.stringify(deger));
}

function baytSay(metin) {
  return new TextEncoder().encode(metin).length;
}

// ---------------------------------------------------------------- sürücüler
// Ortak arayüz (hepsi Promise döner):
//   al(depo, anahtar) -> kayıt | undefined
//   hepsi(depo) -> kayıt[]
//   ekleYoksa(depo, kayit) -> true (eklendi) | false (anahtar zaten var; dokunulmaz)
//   guncelle(depo, anahtar, fn) -> fn(eski|undefined) ne döndürürse o yazılır; undefined dönerse yazılmaz.
//                                  Tek işlem içinde okur-yazar (yarış yok). Yazılan kaydı ya da undefined döner.
// Silme işlemi bilerek yoktur.

const bellekVeritabanlari = new Map(); // ad -> { giden: Map, onbellek: Map, meta: Map, gunluk: Map }

/** Bellek içi yedek sürücü. Aynı ad aynı veriyi görür (IndexedDB gibi); süreç kapanınca veri gider. */
export class BellekSurucu {
  constructor(ad) {
    this.ad = ad;
    this.kalici = false;
    if (!bellekVeritabanlari.has(ad)) {
      bellekVeritabanlari.set(ad, Object.fromEntries(DEPOLAR.map((d) => [d, new Map()])));
    }
    this._v = bellekVeritabanlari.get(ad);
  }

  async al(depo, anahtar) {
    return kopya(this._v[depo].get(anahtar));
  }

  async hepsi(depo) {
    return [...this._v[depo].values()].map(kopya);
  }

  async ekleYoksa(depo, kayit) {
    const anahtar = kayit[ANAHTAR_ALANI[depo]];
    if (this._v[depo].has(anahtar)) return false;
    this._v[depo].set(anahtar, kopya(kayit));
    return true;
  }

  async guncelle(depo, anahtar, fn) {
    // Eşzamanlı (await yok): okuma-yazma arasına başka iş giremez.
    const yeni = fn(kopya(this._v[depo].get(anahtar)));
    if (yeni === undefined) return undefined;
    this._v[depo].set(anahtar, kopya(yeni));
    return kopya(yeni);
  }
}

/** IndexedDB sürücüsü. Veritabanı ilk işlemde açılır. */
export class IdbSurucu {
  constructor(ad, idb) {
    this.ad = ad;
    this.kalici = true;
    this._idb = idb;
    this._acik = null;
  }

  _ac() {
    if (!this._acik) {
      this._acik = new Promise((coz, reddet) => {
        const istek = this._idb.open(this.ad, VT_SURUMU);
        istek.onupgradeneeded = () => {
          const vt = istek.result;
          for (const depo of DEPOLAR) {
            if (!vt.objectStoreNames.contains(depo)) {
              vt.createObjectStore(depo, { keyPath: ANAHTAR_ALANI[depo] });
            }
          }
        };
        istek.onsuccess = () => {
          const vt = istek.result;
          // Başka sekme sürüm yükseltirse bağlantıyı bırak; sonraki işlem yeniden açar.
          vt.onversionchange = () => {
            vt.close();
            this._acik = null;
          };
          coz(vt);
        };
        istek.onerror = () =>
          reddet(new TaaHata('depo', `Yerel veritabanı açılamadı (${this.ad}): ${istek.error?.message ?? 'bilinmiyor'}`));
        istek.onblocked = () =>
          reddet(new TaaHata('depo', `Yerel veritabanı başka bir sekmede kilitli (${this.ad}).`));
      });
      this._acik.catch(() => {
        this._acik = null;
      });
    }
    return this._acik;
  }

  async _islem(depo, kip, is) {
    const vt = await this._ac();
    return new Promise((coz, reddet) => {
      let sonuc;
      const islem = vt.transaction(depo, kip);
      islem.oncomplete = () => coz(sonuc);
      islem.onabort = () =>
        reddet(new TaaHata('depo', `Yerel veritabanı işlemi yarıda kaldı: ${islem.error?.message ?? 'bilinmiyor'}`));
      try {
        is(islem.objectStore(depo), (s) => {
          sonuc = s;
        }, islem);
      } catch (e) {
        try {
          islem.abort();
        } catch {
          /* zaten bitmiş */
        }
        reddet(e);
      }
    });
  }

  al(depo, anahtar) {
    return this._islem(depo, 'readonly', (d, ver) => {
      const i = d.get(anahtar);
      i.onsuccess = () => ver(i.result);
    });
  }

  hepsi(depo) {
    return this._islem(depo, 'readonly', (d, ver) => {
      const i = d.getAll();
      i.onsuccess = () => ver(i.result);
    });
  }

  ekleYoksa(depo, kayit) {
    return this._islem(depo, 'readwrite', (d, ver) => {
      const i = d.add(kayit);
      i.onsuccess = () => ver(true);
      i.onerror = (olay) => {
        if (i.error?.name === 'ConstraintError') {
          olay.preventDefault(); // işlemi düşürme: "zaten var" bir hata değil
          olay.stopPropagation();
          ver(false);
        }
      };
    });
  }

  guncelle(depo, anahtar, fn) {
    return this._islem(depo, 'readwrite', (d, ver, islem) => {
      const i = d.get(anahtar);
      i.onsuccess = () => {
        let yeni;
        try {
          yeni = fn(i.result);
        } catch (e) {
          islem.abort();
          throw e;
        }
        if (yeni === undefined) return;
        const y = d.put(yeni);
        y.onsuccess = () => ver(yeni);
      };
    });
  }
}

// ---------------------------------------------------------------- veritabanı

/**
 * Şube + kullanıcıya özel yerel veritabanını açar. Adı: `taa-<sube>-<kullanici>`.
 * Her şube/kullanıcı ayrı veritabanıdır; biri diğerinin kaydını göremez.
 * Eşzamanlı döner (IndexedDB ilk işlemde açılır); `await acDB(...)` yazmak da zararsızdır.
 * @param {string} sube       `[A-Za-z0-9._]`, 1..48 (tire yok)
 * @param {string} kullanici  `[A-Za-z0-9._]`, 1..48 (tire yok)
 * @param {{surucu?: 'bellek'|'idb', cihaz?: string, saat?: () => number}} [secenek]
 * @returns {{ad: string, kalici: boolean, surucu: object, giden: Giden, onbellek: Onbellek, gunluk: Gunluk}}
 */
export function acDB(sube, kullanici, secenek = {}) {
  if (!parcaGecerli(sube)) {
    throw new TaaHata('gecersiz-sube', 'Şube kimliği yalnız harf, rakam, nokta ve alt çizgi içerebilir (1-48).');
  }
  if (!parcaGecerli(kullanici)) {
    throw new TaaHata('gecersiz-kullanici', 'Kullanıcı adı yalnız harf, rakam, nokta ve alt çizgi içerebilir (1-48).');
  }
  const ad = `taa-${sube}-${kullanici}`;
  const idb = globalThis.indexedDB;
  const tur = secenek.surucu ?? (idb ? 'idb' : 'bellek');
  if (tur === 'idb' && !idb) {
    throw new TaaHata('depo', 'Bu ortamda IndexedDB yok.');
  }
  const surucu = tur === 'idb' ? new IdbSurucu(ad, idb) : new BellekSurucu(ad);
  const db = { ad, kalici: surucu.kalici, surucu };
  db.giden = new Giden(db, { cihaz: secenek.cihaz, saat: secenek.saat });
  db.onbellek = new Onbellek(db, { saat: secenek.saat });
  db.gunluk = new Gunluk(db, { saat: secenek.saat });
  return db;
}

// ---------------------------------------------------------------- olay

let kanal = null;
if (typeof window !== 'undefined' && typeof BroadcastChannel !== 'undefined') {
  // Aynı tarayıcıdaki diğer sekmeler de göstergeyi güncellesin.
  kanal = new BroadcastChannel('taa-giden');
  kanal.onmessage = (m) => yerelYayinla(m.data);
}

function yerelYayinla(ayrinti) {
  const olustur = () =>
    typeof CustomEvent === 'function'
      ? new CustomEvent(OLAY_GIDEN_DEGISTI, { detail: ayrinti })
      : Object.assign(new Event(OLAY_GIDEN_DEGISTI), { detail: ayrinti });
  olaylar.dispatchEvent(olustur());
  if (typeof window !== 'undefined' && typeof window.dispatchEvent === 'function') {
    window.dispatchEvent(olustur());
  }
}

function yayinla(ayrinti) {
  yerelYayinla(ayrinti);
  try {
    kanal?.postMessage(ayrinti);
  } catch {
    /* kanal kapalıysa gösterge yalnız bu sekmede güncellenir */
  }
}

// ---------------------------------------------------------------- giden kutusu

function rastgeleKimlik(uzunluk) {
  const harfler = 'abcdefghijklmnopqrstuvwxyz0123456789';
  const bayt = new Uint8Array(uzunluk);
  globalThis.crypto.getRandomValues(bayt);
  let s = '';
  for (const b of bayt) s += harfler[b % harfler.length];
  return s;
}

/**
 * Giden kutusu. Kayıt biçimi:
 * `{no, sira, program, uc, govde, zaman, durum, gonderim?: {zaman, yanit, yol?}, ret?: {zaman, kod, mesaj, yol?}}`
 * `yol`: kaydı kabul eden / reddeden yolun adı (`merkez` | `sube`); yalnız yol adıyla gönderimde yazılır.
 */
export class Giden {
  constructor(db, { cihaz, saat } = {}) {
    if (cihaz !== undefined && !/^[A-Za-z0-9._]{1,24}$/.test(cihaz)) {
      throw new TaaHata('gecersiz-cihaz', 'Cihaz kimliği yalnız harf, rakam, nokta, alt çizgi içerebilir (1-24).');
    }
    this._db = db;
    this._s = db.surucu;
    this._cihaz = cihaz ?? null;
    this._saat = saat ?? (() => Date.now());
    this._gonderim = null;
  }

  /** Bu veritabanının cihaz kimliği (ilk kullanımda üretilir, meta deposunda kalır). */
  async cihaz() {
    if (this._cihaz) return this._cihaz;
    let k = await this._s.al('meta', 'cihaz');
    if (!k) {
      await this._s.ekleYoksa('meta', { anahtar: 'cihaz', deger: `w${rastgeleKimlik(11)}` });
      k = await this._s.al('meta', 'cihaz'); // iki sekme yarışırsa ilk yazan kazanır
    }
    this._cihaz = k.deger;
    return this._cihaz;
  }

  async _siradaki() {
    const k = await this._s.guncelle('meta', 'sayac', (eski) => ({
      anahtar: 'sayac',
      deger: (eski?.deger ?? 0) + 1,
    }));
    return k.deger;
  }

  /**
   * Kaydı giden kutusuna yazar (ağa dokunmaz).
   * @param {{program: string, uc: string, govde: object, no?: string}} kayit
   *   `no` verilmezse `cihaz-sayaç-zaman` biçiminde üretilir. Aynı işlemi yeniden denerken aynı `no` verilirse
   *   ikinci kayıt oluşmaz; mevcut kaydın durumu `zatenVar: true` ile döner.
   * @returns {Promise<{no: string, durum: string, zatenVar?: true}>}
   */
  async ekle({ program, uc, govde, no } = {}) {
    if (!programGecerli(program)) {
      throw new TaaHata('gecersiz-program', 'Program adı yalnız küçük harf, rakam ve tire içerebilir (1-64).');
    }
    if (!ucGecerli(uc)) {
      throw new TaaHata('gecersiz-uc', 'Uç "/v1/..." biçiminde olmalı; yalnız harf, rakam, /, _ ve - içerebilir.');
    }
    if (govde === null || typeof govde !== 'object' || Array.isArray(govde)) {
      throw new TaaHata('gecersiz-govde', 'Gövde bir nesne ({...}) olmalı.');
    }
    let metin;
    try {
      metin = JSON.stringify(govde);
    } catch {
      throw new TaaHata('gecersiz-govde', 'Gövde JSON olarak yazılamıyor.');
    }
    if (baytSay(metin) > GOVDE_EN_COK) {
      throw new TaaHata('gecersiz-govde', `Gövde çok büyük (sınır ${GOVDE_EN_COK} bayt).`);
    }
    if (no !== undefined && !noGecerli(no)) {
      throw new TaaHata('gecersiz-no', 'Kayıt no yalnız harf, rakam, nokta, alt çizgi ve tire içerebilir (1-64).');
    }
    if (no !== undefined) {
      // Sayaç boşa artmasın: aynı no zaten varsa hiçbir şey yazma.
      const eski = await this._s.al('giden', no);
      if (eski) return { no, durum: eski.durum, zatenVar: true };
    }
    const sira = await this._siradaki();
    const zaman = this._saat();
    const kayitNo = no ?? `${await this.cihaz()}-${sira}-${zaman}`;
    if (!noGecerli(kayitNo)) {
      throw new TaaHata('gecersiz-no', 'Üretilen kayıt no kurala uymuyor.');
    }
    const eklendi = await this._s.ekleYoksa('giden', {
      no: kayitNo,
      sira,
      program,
      uc,
      govde: JSON.parse(metin),
      zaman: new Date(zaman).toISOString(),
      durum: 'bekliyor',
    });
    if (!eklendi) {
      const eski = await this._s.al('giden', kayitNo);
      return { no: kayitNo, durum: eski.durum, zatenVar: true };
    }
    yayinla({ db: this._db.ad, neden: 'eklendi', no: kayitNo });
    return { no: kayitNo, durum: 'bekliyor' };
  }

  /**
   * Kayıtları ekleme sırasıyla döndürür.
   * @param {'bekliyor'|'gonderildi'|'reddedildi'} [durum] verilmezse hepsi
   */
  async liste(durum) {
    if (durum !== undefined && !DURUMLAR.includes(durum)) {
      throw new TaaHata('gecersiz-durum', `Durum şunlardan biri olmalı: ${DURUMLAR.join(', ')}.`);
    }
    const hepsi = await this._s.hepsi('giden');
    return hepsi.filter((k) => durum === undefined || k.durum === durum).sort((a, b) => a.sira - b.sira);
  }

  /** @returns {Promise<{bekliyor: number, gonderildi: number, reddedildi: number}>} */
  async say() {
    const s = { bekliyor: 0, gonderildi: 0, reddedildi: 0 };
    for (const k of await this._s.hepsi('giden')) s[k.durum] += 1;
    return s;
  }

  /**
   * Bekleyenleri ekleme sırasıyla `POST <sunucu><uc>` ile gönderir.
   * - 2xx: `gonderildi`.
   * - 4xx (401, 408, 429 hariç): `reddedildi`; yeniden denenmez, kişi bakar.
   * - Ağ hatası, zaman aşımı, 5xx, 408, 429, yönlendirme: o kayıtta DURUR; kayıt `bekliyor` kalır, sıra bozulmaz.
   * - 401 (oturum geçersiz): DURUR. Süresi dolmuş oturum bütün kutuyu kalıcı "reddedildi" yapmasın diye
   *   bilerek ret sayılmaz; yeniden girişten sonra gönderim kaldığı yerden sürer.
   * İstek: gövde JSON (gövdede `no` yoksa eklenir), başlıklar `Authorization: Bearer <oturum>` ve `X-TAA-No: <no>`.
   * Aynı anda ikinci çağrı yeni gönderim başlatmaz; süren gönderimin sonucunu bekler.
   *
   * İki kullanım:
   * - `gonder({sunucu, oturum})`: tek yol (eski davranış, aynen).
   * - `gonder({yollar, oturum, sube, cevrimici})`: yol listesiyle sıralı deneme, merkez → şube → yerel
   *   (./yol.js `yolluGonder`). Yol başarısızlığı bu veritabanının cihaz günlüğüne yazılır. Sonuca `yol` ve
   *   `denemeler` eklenir.
   * Durunca sonuçta `durmaTuru` da bulunur: `ag` | `zaman-asimi` | `sunucu` | `oturum` | `kilit` | `atlandi`
   * (yolluda ayrıca `cevrimdisi`, `ayar`).
   * @param {{sunucu?: string, yollar?: Array, oturum: string, fetch?: typeof fetch, enCok?: number, zamanAsimiMs?: number,
   *          sube?: string, cevrimici?: boolean}} secenek
   * @returns {Promise<{gonderilen: number, reddedilen: number, kalan: number, durmaNedeni: string|null, durmaKodu?: number, durmaTuru?: string}>}
   */
  gonder(secenek = {}) {
    if (!this._gonderim) {
      const is = Array.isArray(secenek?.yollar)
        ? yolluGonder(this, { gunluk: this._db.gunluk, saat: this._saat, ...secenek })
        : this._kilitliGonder(secenek);
      this._gonderim = is.finally(() => {
        this._gonderim = null;
      });
    }
    return this._gonderim;
  }

  /**
   * Tek bir sunucuya gönderim (yol.js bunu çağırır; `gonder`in süren-iş kilidine girmez). Ek seçenekler:
   * - `yol`: yol adı; kaydın `gonderim.yol` / `ret.yol` alanına yazılır.
   * - `oturumsuz: true`: `Authorization` başlığı gönderilmez (şube düğümü; oturum anahtarı şube ağına çıkmaz).
   * - `basliklar`: ek başlıklar (ör. `X-TAA-Sube`).
   * - `kaliciRet(kod, metin) -> bool`: hangi 4xx kalıcı rettir (verilmezse yukarıdaki kural).
   * - `uygun(kayit) -> bool`: false dönen kayıt bu yola gönderilmez, `bekliyor` kalır (sonuçta `atlanan`).
   */
  tekYolGonder(secenek = {}) {
    return this._kilitliGonder(secenek);
  }

  async _kilitliGonder(secenek) {
    const kilitler = globalThis.navigator?.locks;
    if (!kilitler?.request) return this._gonder(secenek);
    // Başka sekme gönderiyorsa bekleme; çift POST olsa da sunucu aynı no'yu ikinci kez yazmaz.
    return kilitler.request(`taa-giden-${this._db.ad}`, { ifAvailable: true }, async (kilit) => {
      if (!kilit) {
        const s = await this.say();
        return { gonderilen: 0, reddedilen: 0, kalan: s.bekliyor, durmaNedeni: 'Başka bir sekme gönderiyor.', durmaTuru: 'kilit' };
      }
      return this._gonder(secenek);
    });
  }

  async _gonder({
    sunucu,
    oturum,
    fetch: getir = globalThis.fetch,
    enCok = 0,
    zamanAsimiMs = ZAMAN_ASIMI_MS,
    yol,
    oturumsuz = false,
    basliklar,
    kaliciRet,
    uygun,
  } = {}) {
    if (typeof sunucu !== 'string' || !/^https?:\/\/[^\s/?#]+/.test(sunucu)) {
      throw new TaaHata('gecersiz-sunucu', 'Sunucu adresi "https://..." biçiminde olmalı.');
    }
    if (typeof getir !== 'function') {
      throw new TaaHata('fetch-yok', 'Bu ortamda fetch yok; `fetch` seçeneğini verin.');
    }
    if (yol !== undefined && (typeof yol !== 'string' || !/^[a-z0-9-]{1,24}$/.test(yol))) {
      throw new TaaHata('gecersiz-yol', 'Yol adı yalnız küçük harf, rakam ve tire içerebilir (1-24).');
    }
    const ekBasliklar = {};
    for (const [ad, deger] of Object.entries(basliklar ?? {})) {
      if (!/^[A-Za-z0-9-]{1,64}$/.test(ad) || typeof deger !== 'string' || deger === '' || /[\r\n]/.test(deger) || deger.length > 256) {
        throw new TaaHata('gecersiz-baslik', `Ek başlık geçersiz: ${String(ad).slice(0, 64)}`);
      }
      if (/^(authorization|content-type|x-taa-no)$/i.test(ad)) {
        throw new TaaHata('gecersiz-baslik', `Bu başlığı çekirdek kendisi yazar: ${ad}`);
      }
      ekBasliklar[ad] = deger;
    }
    const taban = sunucu.replace(/\/+$/, '');
    const bekleyen = await this.liste('bekliyor');
    const ozet = { gonderilen: 0, reddedilen: 0, kalan: bekleyen.length, durmaNedeni: null };
    if (bekleyen.length === 0) return ozet;
    if (!oturumsuz && (typeof oturum !== 'string' || oturum === '' || /[\r\n]/.test(oturum))) {
      ozet.durmaNedeni = 'Oturum yok; göndermek için giriş yapın.';
      ozet.durmaKodu = 401;
      ozet.durmaTuru = 'oturum';
      return ozet;
    }
    const istek = { taban, oturum: oturumsuz ? null : oturum, basliklar: ekBasliklar, kaliciRet, zamanAsimiMs };

    let degisti = false;
    let atlanan = 0;
    let denenen = 0;
    for (const k of bekleyen) {
      if (typeof uygun === 'function' && !uygun(k)) {
        atlanan += 1;
        continue;
      }
      if (enCok > 0 && denenen >= enCok) break;
      denenen += 1;
      const sonuc = await this._birGonder(getir, istek, k);
      if (sonuc.tur === 'dur') {
        ozet.durmaNedeni = sonuc.neden;
        ozet.durmaTuru = sonuc.durmaTuru;
        if (sonuc.kod !== undefined) ozet.durmaKodu = sonuc.kod;
        break;
      }
      const simdi = new Date(this._saat()).toISOString();
      const yolAlani = yol === undefined ? {} : { yol };
      // Yalnız `bekliyor` kaydı ilerlet: başka sekme çoktan işlediyse üstüne yazma.
      await this._s.guncelle('giden', k.no, (eski) => {
        if (!eski || eski.durum !== 'bekliyor') return undefined;
        return sonuc.tur === 'gonderildi'
          ? { ...eski, durum: 'gonderildi', gonderim: { zaman: simdi, yanit: sonuc.yanit, ...yolAlani } }
          : { ...eski, durum: 'reddedildi', ret: { zaman: simdi, kod: sonuc.kod, mesaj: sonuc.mesaj, ...yolAlani } };
      });
      if (sonuc.tur === 'gonderildi') ozet.gonderilen += 1;
      else ozet.reddedilen += 1;
      ozet.kalan -= 1;
      degisti = true;
    }
    if (atlanan > 0) {
      ozet.atlanan = atlanan;
      if (ozet.durmaNedeni === null) {
        ozet.durmaNedeni = `Bu yola uygun olmayan ${atlanan} kayıt bekliyor.`;
        ozet.durmaTuru = 'atlandi';
      }
    }
    if (degisti) yayinla({ db: this._db.ad, neden: 'gonderildi' });
    return ozet;
  }

  async _birGonder(getir, { taban, oturum, basliklar, kaliciRet, zamanAsimiMs }, k) {
    const govde = Object.prototype.hasOwnProperty.call(k.govde, 'no') ? k.govde : { ...k.govde, no: k.no };
    const denetci = new AbortController();
    const sayac = setTimeout(() => denetci.abort(), zamanAsimiMs);
    let yanit;
    let metin = '';
    try {
      yanit = await getir(`${taban}${k.uc}`, {
        method: 'POST',
        headers: {
          ...basliklar,
          'Content-Type': 'application/json',
          ...(oturum === null ? {} : { Authorization: `Bearer ${oturum}` }),
          'X-TAA-No': k.no,
        },
        body: JSON.stringify(govde),
        redirect: 'error', // taa-kuyruk gibi: yönlendirme izlenmez
        cache: 'no-store',
        credentials: 'omit',
        signal: denetci.signal,
      });
      // Gövde de zaman aşımı içinde okunur; yarım yanıt "gönderildi" sayılmaz.
      metin = typeof yanit.text === 'function' ? await yanit.text() : '';
    } catch (e) {
      return {
        tur: 'dur',
        durmaTuru: denetci.signal.aborted ? 'zaman-asimi' : 'ag',
        neden: denetci.signal.aborted
          ? 'Sunucu zamanında yanıt vermedi.'
          : `Sunucuya ulaşılamadı: ${e?.message ?? 'ağ hatası'}`,
      };
    } finally {
      clearTimeout(sayac);
    }
    const kod = Number(yanit.status) || 0;
    const kisa = metin.trim().slice(0, 300) || 'ayrıntı yok';
    if (kod >= 200 && kod <= 299) {
      let deger = null;
      if (metin.trim() !== '') {
        try {
          deger = JSON.parse(metin);
        } catch {
          deger = kisa;
        }
      }
      return { tur: 'gonderildi', yanit: deger };
    }
    if (kod === 401) {
      return { tur: 'dur', durmaTuru: 'oturum', kod, neden: 'Oturum geçersiz ya da süresi dolmuş; yeniden giriş yapın.' };
    }
    const kalici =
      typeof kaliciRet === 'function' ? kaliciRet(kod, metin) === true : kod >= 400 && kod <= 499 && kod !== 408 && kod !== 429;
    if (kalici && kod >= 400 && kod <= 499) {
      return { tur: 'reddedildi', kod, mesaj: kisa };
    }
    return { tur: 'dur', durmaTuru: 'sunucu', kod, neden: `Sunucu şu an kabul etmiyor (HTTP ${kod}): ${kisa}` };
  }
}

// ---------------------------------------------------------------- önbellek

/** Sunucudan okunan verinin yerel kopyası. Kayıt: `{anahtar, deger, surum, zaman}`. */
export class Onbellek {
  constructor(db, { saat } = {}) {
    this._s = db.surucu;
    this._saat = saat ?? (() => Date.now());
  }

  static _anahtarDenetle(anahtar) {
    if (typeof anahtar !== 'string' || anahtar === '' || anahtar.length > 256) {
      throw new TaaHata('gecersiz-anahtar', 'Önbellek anahtarı 1-256 karakterlik metin olmalı.');
    }
  }

  /** @returns {Promise<any>} değer; yoksa `undefined` */
  async oku(anahtar) {
    return (await this.okuKayit(anahtar))?.deger;
  }

  /** @returns {Promise<{anahtar: string, deger: any, surum: number, zaman: string}|undefined>} */
  async okuKayit(anahtar) {
    Onbellek._anahtarDenetle(anahtar);
    return this._s.al('onbellek', anahtar);
  }

  /**
   * Değeri yazar. `surum` verilmişse ve yereldeki sürüm daha yeniyse YAZMAZ (eski delta yeni veriyi ezmesin).
   * `surum` verilmezse mevcut sürüm korunur (ilk yazmada 0).
   * @returns {Promise<boolean>} yazıldı mı
   */
  async yaz(anahtar, deger, surum) {
    Onbellek._anahtarDenetle(anahtar);
    if (surum !== undefined && (!Number.isSafeInteger(surum) || surum < 0)) {
      throw new TaaHata('gecersiz-surum', 'Sürüm sıfır ya da pozitif tam sayı olmalı.');
    }
    if (deger === undefined) {
      throw new TaaHata('gecersiz-deger', 'Önbelleğe "undefined" yazılamaz.');
    }
    const temiz = kopya(deger);
    const zaman = new Date(this._saat()).toISOString();
    const yazilan = await this._s.guncelle('onbellek', anahtar, (eski) => {
      if (eski && surum !== undefined && eski.surum > surum) return undefined;
      return { anahtar, deger: temiz, surum: surum ?? eski?.surum ?? 0, zaman };
    });
    return yazilan !== undefined;
  }
}

// ---------------------------------------------------------------- cihaz günlüğü

/**
 * Cihaz günlüğü (Yusuf 11-27: günlükler cihazda tutulur, silinmez; merkeze ulaşınca kopyalanır).
 * Yalnız ekleme: satır değişmez, silinmez. Satır: `{sira, zaman, olay, yol?, ayrinti?}`.
 * Kayıt içeriği ve oturum anahtarı günlüğe YAZILMAZ; yazan yalnız olay adı, yol ve kısa ayrıntı verir.
 * "Buraya kadar merkeze kopyalandı" işareti ayrı tutulur (meta `gunluk-kopya`); satırlar yerinde kalır.
 */
export class Gunluk {
  constructor(db, { saat } = {}) {
    this._s = db.surucu;
    this._saat = saat ?? (() => Date.now());
  }

  /**
   * Günlüğe bir satır ekler.
   * @param {{olay: string, yol?: string, ayrinti?: object}} satir  `olay`: `[a-z0-9._-]`, 1-64 (ör. `yol.basarisiz`)
   * @returns {Promise<{sira: number, zaman: string, olay: string, yol?: string, ayrinti?: object}>}
   */
  async yaz({ olay, yol, ayrinti } = {}) {
    if (typeof olay !== 'string' || !/^[a-z0-9._-]{1,64}$/.test(olay)) {
      throw new TaaHata('gecersiz-olay', 'Günlük olayı yalnız küçük harf, rakam, nokta, alt çizgi ve tire içerebilir (1-64).');
    }
    if (yol !== undefined && (typeof yol !== 'string' || !/^[a-z0-9-]{1,24}$/.test(yol))) {
      throw new TaaHata('gecersiz-yol', 'Yol adı yalnız küçük harf, rakam ve tire içerebilir (1-24).');
    }
    const kayit = { sira: 0, zaman: new Date(this._saat()).toISOString(), olay };
    if (yol !== undefined) kayit.yol = yol;
    if (ayrinti !== undefined) {
      if (ayrinti === null || typeof ayrinti !== 'object' || Array.isArray(ayrinti)) {
        throw new TaaHata('gecersiz-ayrinti', 'Günlük ayrıntısı bir nesne ({...}) olmalı.');
      }
      let metin;
      try {
        metin = JSON.stringify(ayrinti);
      } catch {
        throw new TaaHata('gecersiz-ayrinti', 'Günlük ayrıntısı JSON olarak yazılamıyor.');
      }
      if (baytSay(metin) > GUNLUK_AYRINTI_EN_COK) {
        throw new TaaHata('gecersiz-ayrinti', `Günlük ayrıntısı çok büyük (sınır ${GUNLUK_AYRINTI_EN_COK} bayt).`);
      }
      kayit.ayrinti = JSON.parse(metin);
    }
    // Sıra tek işlemde artar (iki sekme aynı sırayı alamaz); yine de çakışırsa bir sonraki denenir.
    for (let deneme = 0; deneme < 5; deneme += 1) {
      const sayac = await this._s.guncelle('meta', 'gunluk-sayac', (eski) => ({
        anahtar: 'gunluk-sayac',
        deger: (eski?.deger ?? 0) + 1,
      }));
      kayit.sira = sayac.deger;
      if (await this._s.ekleYoksa('gunluk', kayit)) return kopya(kayit);
    }
    throw new TaaHata('depo', 'Günlük satırı yazılamadı.');
  }

  /**
   * Satırları sırayla döndürür.
   * @param {{sonra?: number, enCok?: number}} [secenek] `sonra`: bu sıradan büyükler; `enCok`: en çok kaç satır (0 = hepsi)
   */
  async liste({ sonra = 0, enCok = 0 } = {}) {
    const hepsi = (await this._s.hepsi('gunluk')).filter((k) => k.sira > sonra).sort((a, b) => a.sira - b.sira);
    return enCok > 0 ? hepsi.slice(0, enCok) : hepsi;
  }

  /** @returns {Promise<{toplam: number, kopyalanmamis: number}>} */
  async say() {
    const isaret = await this.kopyaIsareti();
    const hepsi = await this._s.hepsi('gunluk');
    return { toplam: hepsi.length, kopyalanmamis: hepsi.filter((k) => k.sira > isaret).length };
  }

  /** Merkeze kopyalanan son satırın sırası (hiç kopyalanmadıysa 0). */
  async kopyaIsareti() {
    return (await this._s.al('meta', 'gunluk-kopya'))?.deger ?? 0;
  }

  /** Kopya işaretini ilerletir (geri gitmez). Satırlara dokunmaz. */
  async kopyalandi(sira) {
    if (!Number.isSafeInteger(sira) || sira < 0) {
      throw new TaaHata('gecersiz-sira', 'Sıra sıfır ya da pozitif tam sayı olmalı.');
    }
    const k = await this._s.guncelle('meta', 'gunluk-kopya', (eski) =>
      (eski?.deger ?? 0) >= sira ? undefined : { anahtar: 'gunluk-kopya', deger: sira },
    );
    return k?.deger ?? (await this.kopyaIsareti());
  }
}
