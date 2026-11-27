// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W2-yol 2026-11-27]
// TAA Web gönderim yolları. Kaynak: PLAN-TAA-WEB-2026-11-27 §9 (Yusuf 11-27), docs/SUBE-DUGUMU.md §13.
// Kural: çevrimiçiyken cihaz DOĞRUDAN merkeze gönderir. Yol sırası: merkez → şube bilgisayarı → cihazın kendi kutusu.
//   - merkez : `POST <merkez><uç>`, oturum anahtarıyla (çekirdeğin bilinen gönderimi).
//   - sube   : `POST <şube düğümü><uç>` (taa-sube-dugum, LAN, varsayılan kapı 8788). Oturum anahtarı düğüme
//              GÖNDERİLMEZ (düz HTTP olabilir); `X-TAA-Sube` eklenir. Düğüm kaydı aynı `no` ile merkeze taşır.
//   - yerel  : kayıt cihazın giden kutusunda `bekliyor` kalır (ağ işi yoktur).
// Yol başarısızlığı cihaz günlüğüne yazılır (yalnız ekleme, silme yok); merkeze ulaşınca günlük kopyalanır.
// Bu dosya çekirdeği içe aktarmaz (döngü olmasın); `giden` ve `gunluk` nesneleri çağırandan gelir. Bağımlılık yok.

export const YOL_ADLARI = Object.freeze(['merkez', 'sube', 'yerel']);
export const SUBE_KAPISI = 8788; // taa-sube-dugum varsayılan kapısı
export const SUBE_GOVDE_EN_COK = 256 * 1024; // düğümün gövde sınırı (413 `govde_buyuk`)
export const UC_GUNLUK_KOPYA = '/v1/gunluk/cihaz'; // ÖNERİ: merkezde karşılığı doğrulanmadı
export const GUNLUK_KOPYA_PARTI = 200;
export const GUNLUK_KOPYA_BEKLEME_MS = 10 * 60 * 1000;

/** Merkezden şubeye geçmeyi haklı çıkaran durma türleri (merkeze ULAŞILAMADI demektir). */
const YOL_DEGISTIREN = new Set(['ag', 'zaman-asimi', 'sunucu']);

function yolHatasi(kod, mesaj) {
  const h = new Error(mesaj);
  h.name = 'TaaHata';
  h.kod = kod;
  return h;
}

/** `http(s)://ad[:kapı]`: yol, sorgu, kullanıcı bilgisi yok. Geçerliyse sondaki `/` atılmış adres, değilse `null`. */
export function adresDuzelt(url) {
  if (typeof url !== 'string') return null;
  const ham = url.trim();
  if (ham === '' || ham.length > 200 || /[\s\\]/.test(ham)) return null;
  const m = /^(https?):\/\/([A-Za-z0-9.-]+|\[[0-9A-Fa-f:]+\])(?::(\d{1,5}))?\/?$/.exec(ham);
  if (!m) return null;
  if (m[3] !== undefined && (Number(m[3]) < 1 || Number(m[3]) > 65535)) return null;
  return `${m[1]}://${m[2].toLowerCase()}${m[3] !== undefined ? `:${Number(m[3])}` : ''}`;
}

/**
 * Yol listesi ayarını çözer. Ayar biçimi (kabuk ve kurulum aynı biçimi yazar):
 *   `[{ad: 'merkez', url}, {ad: 'sube', url[, sube]}, {ad: 'yerel'}]`
 * Sıra ayardan bağımsız olarak her zaman merkez → sube → yerel olur (Yusuf 11-27); `yerel` hep vardır.
 * `merkez` ayarda yoksa ya da adresi bozuksa `secenek.merkez` (kabuğun kendi kökeni) kullanılır.
 * @returns {{yollar: Array<{ad: string, url?: string, sube?: string}>, hatalar: string[]}}
 */
export function yollariCoz(ham, secenek = {}) {
  const hatalar = [];
  const bulunan = {};
  if (ham !== undefined && ham !== null && !Array.isArray(ham)) hatalar.push('Yol listesi bir dizi olmalı.');
  for (const y of Array.isArray(ham) ? ham : []) {
    if (!y || typeof y !== 'object' || !YOL_ADLARI.includes(y.ad)) {
      hatalar.push(`Bilinmeyen yol atlandı: ${typeof y?.ad === 'string' ? y.ad.slice(0, 40) : 'adsız'}.`);
      continue;
    }
    if (bulunan[y.ad]) {
      hatalar.push(`"${y.ad}" yolu iki kez yazılmış; ilki geçerli.`);
      continue;
    }
    if (y.ad === 'yerel') {
      bulunan.yerel = { ad: 'yerel' };
      continue;
    }
    const url = adresDuzelt(y.url);
    if (!url) {
      hatalar.push(`"${y.ad}" yolunun adresi "http://ad:kapı" ya da "https://ad" biçiminde olmalı.`);
      continue;
    }
    bulunan[y.ad] = { ad: y.ad, url };
    if (y.ad === 'sube' && y.sube !== undefined && y.sube !== null && String(y.sube).trim() !== '') {
      const kod = String(y.sube).trim();
      if (/^[A-Za-z0-9._-]{1,48}$/.test(kod)) bulunan.sube.sube = kod.toLowerCase();
      else hatalar.push('Şube kodu yalnız harf, rakam, nokta, alt çizgi ve tire içerebilir (1-48).');
    }
  }
  if (!bulunan.merkez) {
    const url = adresDuzelt(secenek.merkez);
    if (url) bulunan.merkez = { ad: 'merkez', url };
  }
  const yollar = [];
  if (bulunan.merkez) yollar.push(bulunan.merkez);
  if (bulunan.sube) yollar.push(bulunan.sube);
  yollar.push({ ad: 'yerel' });
  return { yollar, hatalar };
}

/** Kişiye gösterilecek yol adı. */
export function yolAdi(ad) {
  return ad === 'merkez' ? 'merkez sunucu' : ad === 'sube' ? 'şube bilgisayarı' : ad === 'yerel' ? 'bu cihaz' : String(ad ?? '');
}

/**
 * Şube düğümünün kalıcı reddi: yalnız 409 `cakisma` (aynı şubede şube bilgisayarındaki hal geçerlidir; kaybeden
 * düğümün çakışma kutusundadır, cihazda `reddedildi` görünür). Düğümün öteki 4xx yanıtları (403 `ag_disi`,
 * 409 `sube_uyusmaz`, 413, 404 `uc_yok` ...) kalıcı ret SAYILMAZ: merkez o kaydı kabul edebilir; kayıt bekler.
 */
export function subeKaliciRet(kod, metin) {
  if (kod !== 409) return false;
  try {
    return JSON.parse(metin)?.hata === 'cakisma';
  } catch {
    return false;
  }
}

function govdeBayti(k) {
  try {
    return new TextEncoder().encode(JSON.stringify({ ...k.govde, no: k.no })).length;
  } catch {
    return Infinity;
  }
}

/**
 * Şube yoluna uygun kayıt süzgeci. Düğümün gövde sınırını aşan kayıt (ör. fotoğraf) düğüme gönderilmez, cihazda
 * bekler. Aynı programın sonraki kayıtları da o turda gönderilmez (program içi sıra bozulmasın: imza, kendi
 * tutanağından önce gitmesin). Başka programların kayıtları etkilenmez.
 */
export function subeSuzgeci(sinir = SUBE_GOVDE_EN_COK) {
  const engelli = new Set();
  return (k) => {
    if (engelli.has(k.program)) return false;
    if (govdeBayti(k) > sinir) {
      engelli.add(k.program);
      return false;
    }
    return true;
  };
}

// Yol durumu yalnız değişince günlüğe yazılır (çevrimdışı bir günde dakikada bir satır birikmesin).
// Anahtar: giden kutusu nesnesi; değer: {merkez: {neden, tekrar}, sube: {...}, yerel: {...}}.
const yolDurumlari = new WeakMap();
const surenGonderimler = new WeakMap();
const kopyaEngeli = new WeakMap(); // gunluk -> zaman (ms): bu ana kadar kopya denenmez

function durumAl(giden) {
  if (!yolDurumlari.has(giden)) yolDurumlari.set(giden, {});
  return yolDurumlari.get(giden);
}

async function gunlugeYaz(gunluk, satir) {
  if (!gunluk || typeof gunluk.yaz !== 'function') return;
  try {
    await gunluk.yaz(satir);
  } catch {
    /* günlük yazılamadı diye gönderim durmaz */
  }
}

async function basarisizlikYaz(giden, gunluk, yol, deneme) {
  const d = durumAl(giden);
  const neden = String(deneme.durmaNedeni ?? 'bilinmiyor').slice(0, 300);
  const anahtar = `${deneme.durmaTuru ?? ''}|${deneme.durmaKodu ?? ''}`;
  if (d[yol] && d[yol].anahtar === anahtar) {
    d[yol].tekrar += 1; // aynı neden sürüyor: satır yazma, say
    return;
  }
  d[yol] = { anahtar, tekrar: 0 };
  const ayrinti = { neden, tur: deneme.durmaTuru ?? null };
  if (deneme.durmaKodu !== undefined) ayrinti.kod = deneme.durmaKodu;
  await gunlugeYaz(gunluk, { olay: 'yol.basarisiz', yol, ayrinti });
}

async function duzeldiYaz(giden, gunluk, yol) {
  const d = durumAl(giden);
  if (!d[yol]) return;
  const tekrar = d[yol].tekrar;
  delete d[yol];
  await gunlugeYaz(gunluk, { olay: 'yol.duzeldi', yol, ayrinti: { bastirilan_tekrar: tekrar } });
}

/**
 * Bekleyen kayıtları yol sırasıyla gönderir. Hiçbir yol kabul etmezse kayıtlar cihazda bekler (kayıp olmaz).
 *
 * Merkezden şubeye yalnız merkeze ULAŞILAMAYINCA geçilir (ağ hatası, zaman aşımı, 5xx/408/429). Merkez yanıt
 * veriyorsa (401 oturum geçersiz dahil) ya da oturum hiç yoksa şube yolu denenmez: çevrimiçiyken kayıt doğrudan
 * merkeze, kişinin kendi oturumuyla gider.
 *
 * @param {object} giden  çekirdeğin `Giden` nesnesi (`tekYolGonder`, `say`, `cihaz`)
 * @param {{yollar: Array, oturum?: string, sube?: string, fetch?: Function, gunluk?: object,
 *          cevrimici?: boolean, enCok?: number, zamanAsimiMs?: number, subeZamanAsimiMs?: number,
 *          saat?: () => number}} secenek
 * @returns {Promise<{gonderilen: number, reddedilen: number, kalan: number, durmaNedeni: string|null,
 *          durmaKodu?: number, durmaTuru?: string, yol: string|null, atlanan?: number,
 *          denemeler: Array<{yol: string, gonderilen: number, reddedilen: number, durmaNedeni: string|null,
 *          durmaTuru?: string, durmaKodu?: number}>}>}
 *   `yol`: kayıt kabul eden son yol; hiçbiri kabul etmediyse ve bekleyen varsa `'yerel'`; iş yoksa `null`.
 */
export function yolluGonder(giden, secenek = {}) {
  if (!giden || typeof giden.tekYolGonder !== 'function') {
    throw yolHatasi('gecersiz-giden', 'Giden kutusu verilmedi.');
  }
  if (!surenGonderimler.has(giden)) {
    const soz = yolla(giden, secenek).finally(() => surenGonderimler.delete(giden));
    surenGonderimler.set(giden, soz);
  }
  return surenGonderimler.get(giden);
}

async function yolla(giden, secenek) {
  const { yollar: hamYollar, oturum, sube, fetch: getir, gunluk, cevrimici = true, enCok, zamanAsimiMs, subeZamanAsimiMs } = secenek;
  const { yollar } = yollariCoz(hamYollar);
  const merkez = yollar.find((y) => y.ad === 'merkez');
  const subeYolu = yollar.find((y) => y.ad === 'sube');
  if (!merkez && !subeYolu) throw yolHatasi('gecersiz-sunucu', 'Yol listesinde merkez ya da şube adresi yok.');

  const ozet = { gonderilen: 0, reddedilen: 0, kalan: (await giden.say()).bekliyor, durmaNedeni: null, yol: null, denemeler: [] };
  const oturumVar = typeof oturum === 'string' && oturum !== '';
  if (ozet.kalan === 0) {
    // Gönderilecek kayıt yok; yine de kopyalanmamış günlük varsa merkeze taşımayı dene (sessizce).
    if (merkez && cevrimici && oturumVar) await gunlukKopyala(giden, gunluk, { merkez: merkez.url, oturum, sube, fetch: getir, saat: secenek.saat });
    return ozet;
  }
  if (!cevrimici) {
    const deneme = { yol: 'yerel', gonderilen: 0, reddedilen: 0, durmaNedeni: 'Çevrimdışı; bağlantı gelince kendiliğinden gönderilecek.', durmaTuru: 'cevrimdisi' };
    await basarisizlikYaz(giden, gunluk, 'merkez', { ...deneme, durmaNedeni: 'Cihaz çevrimdışı (ağ bağlantısı yok).' });
    return { ...ozet, durmaNedeni: deneme.durmaNedeni, durmaTuru: 'cevrimdisi', yol: 'yerel', denemeler: [deneme] };
  }

  const isle = (yol, r) => {
    const deneme = { yol, gonderilen: r.gonderilen, reddedilen: r.reddedilen, durmaNedeni: r.durmaNedeni };
    if (r.durmaTuru !== undefined) deneme.durmaTuru = r.durmaTuru;
    if (r.durmaKodu !== undefined) deneme.durmaKodu = r.durmaKodu;
    if (r.atlanan) deneme.atlanan = r.atlanan;
    ozet.denemeler.push(deneme);
    ozet.gonderilen += r.gonderilen;
    ozet.reddedilen += r.reddedilen;
    ozet.kalan = r.kalan;
    if (r.gonderilen + r.reddedilen > 0) ozet.yol = yol;
    return deneme;
  };
  const durmayiTasi = (d) => {
    ozet.durmaNedeni = d.durmaNedeni;
    if (d.durmaTuru !== undefined) ozet.durmaTuru = d.durmaTuru;
    else delete ozet.durmaTuru;
    if (d.durmaKodu !== undefined) ozet.durmaKodu = d.durmaKodu;
    else delete ozet.durmaKodu;
  };

  let subeyeGec = !merkez;
  if (merkez) {
    const d = isle('merkez', await giden.tekYolGonder({ sunucu: merkez.url, oturum, fetch: getir, enCok, zamanAsimiMs, yol: 'merkez' }));
    if (d.durmaNedeni === null) {
      await duzeldiYaz(giden, gunluk, 'merkez');
      await duzeldiYaz(giden, gunluk, 'yerel');
      await gunlukKopyala(giden, gunluk, { merkez: merkez.url, oturum, sube, fetch: getir, saat: secenek.saat });
      return ozet;
    }
    durmayiTasi(d);
    if (d.durmaTuru === 'kilit') return ozet; // başka sekme gönderiyor: yol hatası değil
    await basarisizlikYaz(giden, gunluk, 'merkez', d);
    if (d.gonderilen + d.reddedilen > 0) {
      // Merkez bu turda yanıt verdi, sonra koptu: günlüğü kopyalamayı yine de dene.
      await gunlukKopyala(giden, gunluk, { merkez: merkez.url, oturum, sube, fetch: getir, saat: secenek.saat });
    }
    subeyeGec = YOL_DEGISTIREN.has(d.durmaTuru);
  }

  if (subeyeGec && subeYolu && ozet.kalan > 0) {
    const kod = (subeYolu.sube ?? (sube === undefined || sube === null ? '' : String(sube))).toLowerCase();
    if (kod === '' || /[\r\n]/.test(kod)) {
      const d = { yol: 'sube', gonderilen: 0, reddedilen: 0, durmaNedeni: 'Şube kodu bilinmiyor; şube bilgisayarına gönderilemez.', durmaTuru: 'ayar' };
      ozet.denemeler.push(d);
      await basarisizlikYaz(giden, gunluk, 'sube', d);
    } else {
      const d = isle(
        'sube',
        await giden.tekYolGonder({
          sunucu: subeYolu.url,
          oturumsuz: true, // oturum anahtarı şube ağına çıkmaz
          basliklar: { 'X-TAA-Sube': kod },
          kaliciRet: subeKaliciRet,
          uygun: subeSuzgeci(),
          fetch: getir,
          enCok,
          zamanAsimiMs: subeZamanAsimiMs ?? zamanAsimiMs,
          yol: 'sube',
        }),
      );
      if (d.durmaTuru === 'kilit') {
        durmayiTasi(d);
        return ozet;
      }
      if (d.durmaNedeni === null || d.durmaTuru === 'atlandi') {
        await duzeldiYaz(giden, gunluk, 'sube');
        if (d.gonderilen + d.reddedilen > 0) {
          await gunlugeYaz(gunluk, {
            olay: 'yol.kullanildi',
            yol: 'sube',
            ayrinti: { gonderilen: d.gonderilen, reddedilen: d.reddedilen, atlanan: d.atlanan ?? 0 },
          });
        }
        if (d.durmaNedeni === null) {
          // Hepsi şube bilgisayarına teslim edildi; merkeze o taşır.
          durmayiTasi(d);
          await duzeldiYaz(giden, gunluk, 'yerel');
          return ozet;
        }
        ozet.atlanan = d.atlanan;
        durmayiTasi(d);
        ozet.durmaNedeni = `${merkez ? 'Merkez sunucuya ulaşılamadı. ' : ''}${d.atlanan} kayıt şube bilgisayarı için fazla büyük; merkez sunucuya ulaşılınca gönderilecek.`;
      } else {
        await basarisizlikYaz(giden, gunluk, 'sube', d);
        if (d.gonderilen + d.reddedilen > 0) {
          await gunlugeYaz(gunluk, { olay: 'yol.kullanildi', yol: 'sube', ayrinti: { gonderilen: d.gonderilen, reddedilen: d.reddedilen, atlanan: d.atlanan ?? 0 } });
        }
        if (!merkez) durmayiTasi(d);
        else ozet.durmaNedeni = `${ozet.durmaNedeni} Şube bilgisayarı: ${d.durmaNedeni}`;
      }
    }
  }

  // Kalan kayıtlar cihazın kendi kutusunda bekler.
  if (ozet.kalan > 0) {
    if (ozet.yol === null) ozet.yol = 'yerel';
    const dy = durumAl(giden);
    if (!dy.yerel) {
      dy.yerel = { anahtar: 'yerel', tekrar: 0 };
      await gunlugeYaz(gunluk, { olay: 'yol.yerel', yol: 'yerel', ayrinti: { bekleyen: ozet.kalan } });
    } else {
      dy.yerel.tekrar += 1;
    }
  }
  return ozet;
}

/**
 * Cihaz günlüğünün kopyalanmamış satırlarını merkeze gönderir (`POST <merkez>/v1/gunluk/cihaz`). Satırlar cihazda
 * KALIR; yalnız "buraya kadar kopyalandı" işareti ilerler. Başarısızlık günlüğe yazılmaz (kendi kendini
 * büyütmesin) ve gönderimi etkilemez; ret/ulaşılamama sonrası bir süre yeniden denenmez.
 * Gövde: `{no, cihaz, sube, ilk, son, satirlar: [{sira, zaman, olay, yol?, ayrinti?}]}`; `no = GNLC-<cihaz>-<ilk>-<son>`.
 * @returns {Promise<{kopyalanan: number, durum: 'tamam'|'yok'|'ertelendi'|'basarisiz', kod?: number}>}
 */
export async function gunlukKopyala(giden, gunluk, { merkez, oturum, sube, fetch: getir = globalThis.fetch, saat, parti = GUNLUK_KOPYA_PARTI, enCokParti = 5 } = {}) {
  const sonuc = { kopyalanan: 0, durum: 'yok' };
  if (!gunluk || typeof gunluk.liste !== 'function' || typeof gunluk.kopyalandi !== 'function') return sonuc;
  const taban = adresDuzelt(merkez);
  if (!taban || typeof oturum !== 'string' || oturum === '' || /[\r\n]/.test(oturum) || typeof getir !== 'function') return sonuc;
  const simdi = (saat ?? Date.now)();
  if ((kopyaEngeli.get(gunluk) ?? 0) > simdi) return { ...sonuc, durum: 'ertelendi' };
  try {
    const cihaz = await giden.cihaz();
    for (let tur = 0; tur < enCokParti; tur += 1) {
      const isaret = await gunluk.kopyaIsareti();
      const satirlar = await gunluk.liste({ sonra: isaret, enCok: parti });
      if (satirlar.length === 0) break;
      const ilk = satirlar[0].sira;
      const son = satirlar[satirlar.length - 1].sira;
      const no = `GNLC-${cihaz}-${ilk}-${son}`;
      const govde = { no, cihaz, ilk, son, satirlar };
      if (sube !== undefined && sube !== null && String(sube) !== '') govde.sube = String(sube).toLowerCase();
      const denetci = new AbortController();
      const sayac = setTimeout(() => denetci.abort(), 20_000);
      let yanit;
      try {
        yanit = await getir(`${taban}${UC_GUNLUK_KOPYA}`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${oturum}`, 'X-TAA-No': no },
          body: JSON.stringify(govde),
          redirect: 'error',
          cache: 'no-store',
          credentials: 'omit',
          signal: denetci.signal,
        });
        if (typeof yanit.text === 'function') await yanit.text();
      } finally {
        clearTimeout(sayac);
      }
      const kod = Number(yanit.status) || 0;
      if (kod < 200 || kod > 299) {
        kopyaEngeli.set(gunluk, simdi + GUNLUK_KOPYA_BEKLEME_MS);
        return { ...sonuc, durum: 'basarisiz', kod };
      }
      await gunluk.kopyalandi(son);
      sonuc.kopyalanan += satirlar.length;
      sonuc.durum = 'tamam';
    }
  } catch {
    kopyaEngeli.set(gunluk, simdi + GUNLUK_KOPYA_BEKLEME_MS);
    return { ...sonuc, durum: 'basarisiz' };
  }
  return sonuc;
}
