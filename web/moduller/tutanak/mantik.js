// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W3-tutanak 2026-11-27]
// Tutaa (tutanak) web modülünün saf kuralları: şablon okuma, tutanak doğrulama, fotoğraf (OCR sırası), liste, imza.
// Kaynak: PLAN-TAA-WEB-2026-11-27 §3 ve §9; Tutaa `API-SOZLESMESI.md` (POST /v1/tutanak {sablon, surum, alanlar,
// satirlar, sha256, olusturma}); şablon biçimi crates/taa-kasa/sablonlar/kasa-cift-dogrulama.json ile aynı
// (kimlik, ad, surum, bolumler[].alanlar[], tablo, imzalar[]).
// OCR burada YOK: fotoğraf giden kutusuna `ocr_bekliyor` olarak yazılır, sonuç sunucuda üretilir.
// Kasa Çift Doğrulama Tutanağı burada YOK: TAA Kasa'ya taşındı (DEGISIKLIK.md, T6b iptal).
// Buradaki denetim ön denetimdir; asıl karar sunucudadır. DOM ve ağ yok.

import { tarihGecerli, eslesir } from '../ortak/bicim.js';

export const PROGRAM = 'tutanak';

/** Sunucu uçları. `/v1/tutanak` Tutaa API sözleşmesindedir; `imza` ve `foto` VARSAYIMDIR (tek yerden değişir). */
export const UC = { olustur: '/v1/tutanak', imza: '/v1/tutanak/imza', foto: '/v1/tutanak/foto' };
export const SOZLESME = { olustur: 'tutanak.olustur@1', imza: 'tutanak.imza@1', foto: 'tutanak.foto@1' };

/** İzin adları (baglam.yetki). VARSAYIM: sunucudaki adlar kesinleşince yalnız burası değişir. */
export const IZIN = { oku: 'tutanak.oku', olustur: 'tutanak.olustur', imza: 'tutanak.imza', foto: 'tutanak.foto' };

export const ANAHTAR = { sablonlar: 'tutanak/sablonlar', liste: 'tutanak/liste' };

/** TAA Kasa'ya taşınan şablonlar: önbellekte gelse de bu modülde gösterilmez. */
export const KASADA_OLAN = Object.freeze(['kasa-cift-dogrulama']);

export const OCR_BEKLIYOR = 'ocr_bekliyor';
export const FOTO_TURLERI = Object.freeze(['image/jpeg', 'image/png', 'image/webp']);
/** Bir fotoğrafın en çok ham boyutu. base64 (4/3) + zarf, çekirdeğin 1 MiB gövde sınırının altında kalır. */
export const FOTO_EN_COK = 700 * 1024;
/** Bir tutanak için en çok fotoğraf (Tutaa API: "≤3 foto"). */
export const FOTO_EN_COK_ADET = 3;
/** Küçültmede uzun kenar (piksel); OCR için okunaklı kalır. */
export const FOTO_UZUN_KENAR = 1600;

export const METIN_EN_COK = 2000;
export const UZUN_METIN_EN_COK = 5000;
export const SATIR_EN_COK = 60;
export const CIZIM_EN_AZ_NOKTA = 8;
export const CIZIM_EN_COK_NOKTA = 3000;
export const CIZIM_OLCU = { en: 600, boy: 200 };
export const VARSAYILAN_ROLLER = Object.freeze(['Düzenleyen', 'Tanık', 'İlgili']);

const TURLER = ['metin', 'uzunmetin', 'sayi', 'tarih', 'saat', 'secim', 'coklu', 'onay'];
const kimlikGecerli = (k) => typeof k === 'string' && /^[A-Za-z0-9_-]{1,64}$/.test(k);
const metin = (d) => (typeof d === 'string' ? d.trim() : '');

// ---------------------------------------------------------------- şablonlar

function alanDuzelt(a) {
  if (!a || !kimlikGecerli(a.id)) return null;
  const tur = TURLER.includes(a.tur) ? a.tur : 'metin';
  const alan = { id: a.id, etiket: metin(a.etiket) || a.id, tur, zorunlu: a.zorunlu === true };
  if (tur === 'secim' || tur === 'coklu') {
    alan.secenekler = Array.isArray(a.secenekler) ? a.secenekler.filter((s) => typeof s === 'string' && s !== '') : [];
    if (alan.secenekler.length === 0) alan.tur = 'metin';
  }
  return alan;
}

function sablonDuzelt(s) {
  if (!s || !kimlikGecerli(s.kimlik) || KASADA_OLAN.includes(s.kimlik)) return null;
  const surum = Number.isSafeInteger(s.surum) && s.surum > 0 ? s.surum : 1;
  const gorulen = new Set();
  const bolumler = [];
  for (const b of Array.isArray(s.bolumler) ? s.bolumler : []) {
    const alanlar = (Array.isArray(b?.alanlar) ? b.alanlar : []).map(alanDuzelt).filter((a) => a && !gorulen.has(a.id) && gorulen.add(a.id));
    if (alanlar.length > 0) bolumler.push({ baslik: metin(b.baslik), alanlar });
  }
  let tablo = null;
  if (s.tablo && Array.isArray(s.tablo.sutunlar)) {
    const sg = new Set();
    const sutunlar = s.tablo.sutunlar
      .map(alanDuzelt)
      .filter((a) => a && !sg.has(a.id) && sg.add(a.id))
      .map((a) => (a.tur === 'coklu' || a.tur === 'uzunmetin' ? { id: a.id, etiket: a.etiket, tur: 'metin', zorunlu: a.zorunlu } : a)); // hücrede çoklu seçim yok
    if (sutunlar.length > 0) {
      const sabit = (Array.isArray(s.tablo.sabitSatirlar) ? s.tablo.sabitSatirlar : []).filter((x) => typeof x === 'string').slice(0, SATIR_EN_COK);
      const enAz = Number.isSafeInteger(s.tablo.enAzSatir) ? Math.min(Math.max(s.tablo.enAzSatir, 0), SATIR_EN_COK) : 0;
      tablo = { baslik: metin(s.tablo.baslik), sutunlar, sabitSatirlar: sabit, enAzSatir: Math.max(enAz, sabit.length) };
    }
  }
  if (bolumler.length === 0 && !tablo) return null;
  const imzalar = [];
  for (const i of Array.isArray(s.imzalar) ? s.imzalar : []) {
    const rol = metin(i?.rol);
    if (rol !== '' && rol.length <= 80 && !imzalar.some((x) => x.rol === rol)) imzalar.push({ rol, imtinaKaydi: i.imtinaKaydi === true });
  }
  return { kimlik: s.kimlik, ad: metin(s.ad) || s.kimlik, surum, baslik: metin(s.baslik) || metin(s.ad) || s.kimlik, bolumler, tablo, imzalar };
}

/**
 * Önbellekten gelen şablonları denetler: `[...]` ya da `{sablonlar: [...]}`. Bozuk şablon, alanı olmayan şablon ve
 * Kasa'ya taşınan şablon (çift doğrulama) atılır. Ada göre sıralanır.
 */
export function sablonlariDuzelt(ham) {
  const l = Array.isArray(ham) ? ham : ham && Array.isArray(ham.sablonlar) ? ham.sablonlar : [];
  const gorulen = new Set();
  return l
    .map(sablonDuzelt)
    .filter((s) => s && !gorulen.has(s.kimlik) && gorulen.add(s.kimlik))
    .sort((a, b) => a.ad.localeCompare(b.ad, 'tr'));
}

/** Şablonun imza rolleri; şablon yoksa ya da rol tanımlamıyorsa varsayılan roller. */
export function sablonRolleri(sablon) {
  return sablon && sablon.imzalar.length > 0 ? sablon.imzalar.map((i) => i.rol) : [...VARSAYILAN_ROLLER];
}

// ---------------------------------------------------------------- özet (sha256) ve kodlama

/** Anahtarları sıralı, boşluksuz JSON: aynı içerik her cihazda aynı özeti verir. */
export function kanonik(deger) {
  if (Array.isArray(deger)) return `[${deger.map(kanonik).join(',')}]`;
  if (deger && typeof deger === 'object') {
    return `{${Object.keys(deger)
      .sort()
      .map((k) => `${JSON.stringify(k)}:${kanonik(deger[k])}`)
      .join(',')}}`;
  }
  return JSON.stringify(deger ?? null);
}

/** SHA-256, küçük harf onaltılık. `veri`: metin (UTF-8) ya da Uint8Array. WebCrypto yoksa hata fırlatır. */
export async function sha256Hex(veri) {
  const kripto = globalThis.crypto?.subtle;
  if (!kripto) throw new Error('Bu tarayıcıda özet (SHA-256) hesaplanamıyor; güvenli bağlantı (https) gerekir.');
  const bayt = typeof veri === 'string' ? new TextEncoder().encode(veri) : veri;
  const ozet = new Uint8Array(await kripto.digest('SHA-256', bayt));
  return [...ozet].map((b) => b.toString(16).padStart(2, '0')).join('');
}

/** Uint8Array → base64 (parça parça; büyük dizide yığın taşmasın). */
export function base64Yaz(bayt) {
  let ikili = '';
  for (let i = 0; i < bayt.length; i += 0x8000) ikili += String.fromCharCode(...bayt.subarray(i, i + 0x8000));
  return btoa(ikili);
}

// ---------------------------------------------------------------- tutanak oluşturma

function alanDegeri(alan, ham, hatalar, onek = '') {
  const ad = `${onek}${alan.etiket}`;
  if (alan.tur === 'onay') return ham === true;
  if (alan.tur === 'coklu') {
    const secilen = Array.isArray(ham) ? alan.secenekler.filter((s) => ham.includes(s)) : [];
    if (alan.zorunlu && secilen.length === 0) hatalar.push(`${ad}: en az bir seçenek işaretlenmeli`);
    return secilen;
  }
  const m = metin(ham === undefined || ham === null ? '' : String(ham));
  if (m === '') {
    if (alan.zorunlu) hatalar.push(`${ad}: boş bırakılamaz`);
    return null;
  }
  if (alan.tur === 'sayi') {
    // "1.234,56", "1234,5", "1234.5", "-45": binlik nokta yalnız virgül de varsa binliktir.
    const sade = m.includes(',') ? m.replace(/\./g, '').replace(',', '.') : m;
    if (!/^-?\d{1,15}(\.\d{1,4})?$/.test(sade)) {
      hatalar.push(`${ad}: sayı olmalı (ör. 1250 ya da 1.250,50)`);
      return null;
    }
    return Number(sade);
  }
  if (alan.tur === 'tarih') {
    if (!tarihGecerli(m)) {
      hatalar.push(`${ad}: tarih YYYY-AA-GG olmalı`);
      return null;
    }
    return m;
  }
  if (alan.tur === 'saat') {
    if (!/^([01]\d|2[0-3]):[0-5]\d$/.test(m)) {
      hatalar.push(`${ad}: saat SS:DD olmalı`);
      return null;
    }
    return m;
  }
  if (alan.tur === 'secim') {
    if (!alan.secenekler.includes(m)) {
      hatalar.push(`${ad}: listedeki seçeneklerden biri olmalı`);
      return null;
    }
    return m;
  }
  const sinir = alan.tur === 'uzunmetin' ? UZUN_METIN_EN_COK : METIN_EN_COK;
  if (m.length > sinir) {
    hatalar.push(`${ad}: en çok ${sinir} karakter olabilir`);
    return null;
  }
  return m;
}

const bosDeger = (d) => d === null || d === false || (Array.isArray(d) && d.length === 0);

/**
 * Formdan gelen değerleri şablona göre doğrular ve gövdeyi hazırlar (özet hariç; bkz. `ozetEkle`).
 * `girdi`: `{alanlar: {id: değer}, satirlar: [{sutunId: değer}]}`. Tamamen boş tablo satırı atılır.
 * Belge numarası istemcide ÜRETİLMEZ: numarayı sunucu (sıra tutan) verir; kayıt giden `no` ile tekildir.
 * @returns {{hatalar: string[], govde?: object}}
 */
export function tutanakHazirla(sablon, girdi, zamanMs = Date.now()) {
  const hatalar = [];
  if (!sablon || !kimlikGecerli(sablon.kimlik)) return { hatalar: ['şablon seçilmedi'] };
  if (KASADA_OLAN.includes(sablon.kimlik)) return { hatalar: ['bu tutanak TAA Kasa programında düzenlenir'] };
  const ham = girdi && typeof girdi === 'object' ? girdi : {};
  const alanlar = {};
  for (const b of sablon.bolumler) {
    for (const a of b.alanlar) {
      const d = alanDegeri(a, ham.alanlar ? ham.alanlar[a.id] : undefined, hatalar);
      if (!bosDeger(d)) alanlar[a.id] = d;
    }
  }
  const satirlar = [];
  if (sablon.tablo) {
    const gelen = Array.isArray(ham.satirlar) ? ham.satirlar : [];
    if (gelen.length > SATIR_EN_COK) hatalar.push(`tablo en çok ${SATIR_EN_COK} satır olabilir`);
    gelen.slice(0, SATIR_EN_COK).forEach((satir, i) => {
      const s = {};
      const satirHatalari = [];
      // Sabit satırın ilk sütunu (kalem adı) şablondan gelir; tek başına "dolu satır" sayılmaz.
      const sabitAd = sablon.tablo.sabitSatirlar[i];
      let dolu = false;
      sablon.tablo.sutunlar.forEach((sutun, j) => {
        const girilen = satir ? satir[sutun.id] : undefined;
        const d = alanDegeri({ ...sutun, zorunlu: false }, girilen, satirHatalari, `Satır ${i + 1}, `);
        if (bosDeger(d)) return;
        s[sutun.id] = d;
        if (!(j === 0 && sabitAd !== undefined && d === sabitAd)) dolu = true;
      });
      if (!dolu && satirHatalari.length === 0) return;
      hatalar.push(...satirHatalari);
      for (const sutun of sablon.tablo.sutunlar) {
        if (sutun.zorunlu && s[sutun.id] === undefined) hatalar.push(`Satır ${i + 1}, ${sutun.etiket}: boş bırakılamaz`);
      }
      satirlar.push(s);
    });
  }
  if (Object.keys(alanlar).length === 0 && satirlar.length === 0 && hatalar.length === 0) hatalar.push('tutanak boş; en az bir alan doldurulmalı');
  if (hatalar.length) return { hatalar };
  return {
    hatalar,
    govde: { sozlesme: SOZLESME.olustur, sablon: sablon.kimlik, surum: sablon.surum, alanlar, satirlar, olusturma: new Date(zamanMs).toISOString() },
  };
}

/** Gövdeye içerik özetini (`sha256`) ekler: `{sablon, surum, alanlar, satirlar}` kanonik JSON'unun SHA-256'sı. */
export async function ozetEkle(govde) {
  const { sablon, surum, alanlar, satirlar } = govde;
  return { ...govde, sha256: await sha256Hex(kanonik({ sablon, surum, alanlar, satirlar })) };
}

// ---------------------------------------------------------------- fotoğraf (OCR sırası)

/** Dosyanın ilk baytlarına bakarak gerçek türünü bulur (uzantıya / beyana güvenilmez). Bilinmiyorsa `null`. */
export function fotoTuru(bayt) {
  if (!bayt || bayt.length < 12) return null;
  if (bayt[0] === 0xff && bayt[1] === 0xd8 && bayt[2] === 0xff) return 'image/jpeg';
  if (bayt[0] === 0x89 && bayt[1] === 0x50 && bayt[2] === 0x4e && bayt[3] === 0x47) return 'image/png';
  const yazi = (b, u) => String.fromCharCode(...bayt.subarray(b, b + u));
  if (yazi(0, 4) === 'RIFF' && yazi(8, 4) === 'WEBP') return 'image/webp';
  return null;
}

/** Küçültme ölçüsü: uzun kenar `enUzun`u aşmaz, oran korunur, büyütme yapılmaz. */
export function kucultmeOlcusu(en, boy, enUzun = FOTO_UZUN_KENAR) {
  if (!(en > 0) || !(boy > 0)) return null;
  const oran = Math.min(1, enUzun / Math.max(en, boy));
  return { en: Math.max(1, Math.round(en * oran)), boy: Math.max(1, Math.round(boy * oran)) };
}

function dosyaAdi(ad, tur) {
  const uzanti = { 'image/jpeg': 'jpg', 'image/png': 'png', 'image/webp': 'webp' }[tur];
  const govde = String(ad ?? '')
    .replace(/\.[A-Za-z0-9]{1,5}$/, '')
    .replace(/[^A-Za-z0-9._-]+/g, '_')
    .replace(/^[._]+/, '')
    .slice(0, 60);
  return `${govde || 'foto'}.${uzanti}`;
}

/**
 * Fotoğrafı giden kutusu gövdesine çevirir: `durum: 'ocr_bekliyor'`. Çözümleme (OCR) burada yapılmaz; sunucu yapar.
 * @param {{ad?: string, bayt: Uint8Array}} dosya
 * @param {{grup: string, sayfa: number, sayfaSayisi: number, sablon?: string, not?: string}} bilgi
 *   `grup`: aynı tutanağın sayfalarını bağlayan kimlik; `sablon`: belge türü biliniyorsa şablon kimliği.
 * @returns {Promise<{hatalar: string[], govde?: object}>}
 */
export async function fotoHazirla(dosya, bilgi = {}, zamanMs = Date.now()) {
  const hatalar = [];
  const bayt = dosya?.bayt;
  const ad = `Fotoğraf ${bilgi.sayfa ?? 1}`;
  if (!(bayt instanceof Uint8Array) || bayt.length === 0) return { hatalar: [`${ad}: dosya okunamadı`] };
  const tur = fotoTuru(bayt);
  if (!tur) hatalar.push(`${ad}: yalnız JPEG, PNG ya da WebP fotoğraf kabul edilir`);
  if (bayt.length > FOTO_EN_COK) hatalar.push(`${ad}: çok büyük (${Math.ceil(bayt.length / 1024)} KB; sınır ${FOTO_EN_COK / 1024} KB)`);
  if (!kimlikGecerli(bilgi.grup)) hatalar.push('fotoğraf grubu kimliği yok');
  const { sayfa, sayfaSayisi } = bilgi;
  if (!Number.isSafeInteger(sayfa) || !Number.isSafeInteger(sayfaSayisi) || sayfa < 1 || sayfa > sayfaSayisi || sayfaSayisi > FOTO_EN_COK_ADET) {
    hatalar.push(`bir tutanak için en çok ${FOTO_EN_COK_ADET} fotoğraf gönderilebilir`);
  }
  if (bilgi.sablon !== undefined && bilgi.sablon !== '' && (!kimlikGecerli(bilgi.sablon) || KASADA_OLAN.includes(bilgi.sablon))) {
    hatalar.push('belge türü geçersiz');
  }
  const not = metin(bilgi.not);
  if (not.length > 300) hatalar.push('not en çok 300 karakter olabilir');
  if (hatalar.length) return { hatalar };
  const govde = {
    sozlesme: SOZLESME.foto,
    durum: OCR_BEKLIYOR,
    grup: bilgi.grup,
    sayfa,
    sayfa_sayisi: sayfaSayisi,
    cekim: new Date(zamanMs).toISOString(),
    dosya: { ad: dosyaAdi(dosya.ad, tur), tur, boyut: bayt.length, sha256: await sha256Hex(bayt), veri_b64: base64Yaz(bayt) },
  };
  if (bilgi.sablon) govde.sablon = bilgi.sablon;
  if (not !== '') govde.not = not;
  return { hatalar, govde };
}

// ---------------------------------------------------------------- liste

const durumAdlari = { acik: 'Açık', iptal: 'İptal', [OCR_BEKLIYOR]: 'OCR bekliyor', taslak: 'Taslak' };
export const durumAdi = (d) => durumAdlari[d] || String(d || '');

/**
 * Ekranda gösterilecek tutanak listesi: sunucudan gelen özetler (önbellek `tutanak/liste`) + giden kutusundaki
 * kayıtlar (yeni tutanak, fotoğraf grupları, imzalar). Öğe:
 * `{anahtar, id?, istemci_no?, no, sablon, tur: 'tutanak'|'foto', durum, tarih, baslik, imzalar[], sha256?,
 *   bekliyor?, reddedildi?, ret?, sayfa?: {toplam, bekleyen}}`.
 * Sunucu özeti `istemci_no` (ya da fotoğraf için `grup`) taşıyorsa yerel kayıtla birleştirilir; taşımıyorsa
 * gönderilmiş yerel kayıt "sunucu listesi bekleniyor" notuyla ayrı görünür.
 */
export function tutanakListesi(onbellekListe, kayitlar) {
  const ham = Array.isArray(onbellekListe) ? onbellekListe : onbellekListe && Array.isArray(onbellekListe.tutanaklar) ? onbellekListe.tutanaklar : [];
  const liste = [];
  const istemciNolari = new Set();
  const gruplar = new Set();
  for (const t of ham) {
    if (!t || (t.id === undefined && !t.istemci_no)) continue;
    if (KASADA_OLAN.includes(t.sablon)) continue;
    if (t.istemci_no) istemciNolari.add(String(t.istemci_no));
    if (t.grup) gruplar.add(String(t.grup));
    liste.push({
      anahtar: t.id !== undefined ? `id:${t.id}` : `no:${t.istemci_no}`,
      id: t.id,
      istemci_no: t.istemci_no ? String(t.istemci_no) : undefined,
      no: metin(t.no),
      sablon: metin(t.sablon),
      tur: t.durum === OCR_BEKLIYOR ? 'foto' : 'tutanak',
      durum: metin(t.durum) || 'acik',
      tarih: metin(t.tarih) || metin(t.olusturma).slice(0, 10),
      baslik: metin(t.baslik) || metin(t.ad),
      sha256: metin(t.sha256) || undefined,
      imzalar: Array.isArray(t.imzalar) ? t.imzalar.filter((i) => i && metin(i.rol)).map((i) => ({ rol: metin(i.rol), imzalayan: metin(i.imzalayan), imtina: i.imtina === true })) : [],
    });
  }
  const yerel = new Map(); // istemci_no -> öğe
  const fotoGruplari = new Map();
  for (const k of kayitlar || []) {
    const g = k && k.govde;
    if (!g || k.program !== PROGRAM) continue;
    if (k.uc === UC.olustur) {
      if (istemciNolari.has(k.no) && k.durum === 'gonderildi') continue; // sunucu listesinde zaten var
      const oge = {
        anahtar: `no:${k.no}`,
        istemci_no: k.no,
        no: '',
        sablon: metin(g.sablon),
        tur: 'tutanak',
        durum: 'acik',
        tarih: metin(g.olusturma).slice(0, 10),
        baslik: '',
        sha256: metin(g.sha256) || undefined,
        imzalar: [],
      };
      if (k.durum === 'bekliyor') oge.bekliyor = true;
      else if (k.durum === 'reddedildi') Object.assign(oge, { reddedildi: true, ret: k.ret ? `HTTP ${k.ret.kod}: ${k.ret.mesaj}` : '' });
      else oge.listeBekliyor = true;
      yerel.set(k.no, oge);
      liste.push(oge);
    } else if (k.uc === UC.foto && typeof g.grup === 'string') {
      if (gruplar.has(g.grup) && k.durum === 'gonderildi') continue;
      let oge = fotoGruplari.get(g.grup);
      if (!oge) {
        oge = {
          anahtar: `grup:${g.grup}`,
          grup: g.grup,
          no: '',
          sablon: metin(g.sablon),
          tur: 'foto',
          durum: OCR_BEKLIYOR,
          tarih: metin(g.cekim).slice(0, 10),
          baslik: metin(g.not),
          imzalar: [],
          sayfa: { toplam: Number.isSafeInteger(g.sayfa_sayisi) ? g.sayfa_sayisi : 1, bekleyen: 0, reddedilen: 0 },
        };
        fotoGruplari.set(g.grup, oge);
        liste.push(oge);
      }
      if (k.durum === 'bekliyor') {
        oge.sayfa.bekleyen += 1;
        oge.bekliyor = true;
      } else if (k.durum === 'reddedildi') {
        oge.sayfa.reddedilen += 1;
        oge.reddedildi = true;
        oge.ret = k.ret ? `HTTP ${k.ret.kod}: ${k.ret.mesaj}` : '';
      }
    }
  }
  for (const k of kayitlar || []) {
    const g = k && k.govde;
    if (!g || k.program !== PROGRAM || k.uc !== UC.imza || k.durum === 'gonderildi') continue;
    const hedef = g.tutanak_id !== undefined ? liste.find((o) => o.id === g.tutanak_id) : yerel.get(g.istemci_no) || liste.find((o) => o.istemci_no === g.istemci_no);
    if (!hedef) continue;
    if (k.durum === 'bekliyor' && !hedef.imzalar.some((i) => i.rol === g.rol)) {
      hedef.imzalar.push({ rol: metin(g.rol), imzalayan: metin(g.imzalayan), imtina: g.imtina === true, bekliyor: true });
    } else if (k.durum === 'reddedildi') {
      hedef.reddedilenImza = metin(g.rol);
    }
  }
  return liste.sort((a, b) => (a.tarih < b.tarih ? 1 : a.tarih > b.tarih ? -1 : a.anahtar.localeCompare(b.anahtar)));
}

/** Tutanağın henüz imzalanmamış rolleri (iptal, reddedilmiş ve fotoğraf kayıtlarında boş). */
export function eksikRoller(tutanak, sablon) {
  if (!tutanak || tutanak.tur !== 'tutanak' || tutanak.durum === 'iptal' || tutanak.reddedildi) return [];
  return sablonRolleri(sablon).filter((r) => !tutanak.imzalar.some((i) => i.rol === r));
}

export const SUZGECLER = [
  ['hepsi', 'Hepsi'],
  ['imza', 'İmza bekleyen'],
  ['gonderilmeyen', 'Gönderilmeyen'],
  ['ocr', 'OCR bekleyen'],
  ['reddedilen', 'Reddedilen'],
];

/** Süzgeç + arama (numara, şablon adı, başlık, imzalayan). `sablonlar`: şablon adını ve rollerini bulmak için. */
export function suz(liste, suzgec, sorgu, sablonlar = []) {
  const sablonu = (t) => sablonlar.find((s) => s.kimlik === t.sablon);
  let l = liste;
  if (suzgec === 'imza') l = l.filter((t) => eksikRoller(t, sablonu(t)).length > 0);
  else if (suzgec === 'gonderilmeyen') l = l.filter((t) => t.bekliyor || t.imzalar.some((i) => i.bekliyor));
  else if (suzgec === 'ocr') l = l.filter((t) => t.durum === OCR_BEKLIYOR);
  else if (suzgec === 'reddedilen') l = l.filter((t) => t.reddedildi || t.reddedilenImza);
  if (metin(sorgu) !== '') {
    l = l.filter((t) => eslesir([t.no, t.istemci_no, t.sablon, sablonu(t)?.ad, t.baslik, t.tarih, ...t.imzalar.map((i) => i.imzalayan)].join(' '), sorgu));
  }
  return [...l];
}

// ---------------------------------------------------------------- imza

/**
 * Elle atılan imzanın çizimini denetler ve sadeleştirir. Çizim görüntü değil, çizgi noktalarıdır:
 * `{en, boy, cizgiler: [[x0, y0, x1, y1, ...], ...]}` (tam sayı piksel, tuval ölçüsünde).
 * Çok uzun çizim seyreltilir (her çizginin ilk ve son noktası korunur). Geçersizse `null`.
 */
export function cizimDuzelt(ham) {
  if (!ham || !Array.isArray(ham.cizgiler)) return null;
  const en = Number.isSafeInteger(ham.en) && ham.en > 0 && ham.en <= 4000 ? ham.en : null;
  const boy = Number.isSafeInteger(ham.boy) && ham.boy > 0 && ham.boy <= 4000 ? ham.boy : null;
  if (!en || !boy) return null;
  let cizgiler = [];
  for (const c of ham.cizgiler) {
    if (!Array.isArray(c) || c.length < 2 || c.length % 2 !== 0 || !c.every(Number.isFinite)) continue;
    cizgiler.push(c.map((v, i) => Math.min(Math.max(Math.round(v), 0), i % 2 === 0 ? en : boy)));
  }
  let nokta = cizgiler.reduce((t, c) => t + c.length / 2, 0);
  if (nokta < CIZIM_EN_AZ_NOKTA) return null;
  if (nokta > CIZIM_EN_COK_NOKTA) {
    const adim = Math.ceil(nokta / CIZIM_EN_COK_NOKTA);
    cizgiler = cizgiler.map((c) => {
      const y = [];
      const n = c.length / 2;
      for (let i = 0; i < n; i += 1) if (i % adim === 0 || i === n - 1) y.push(c[2 * i], c[2 * i + 1]);
      return y;
    });
    nokta = cizgiler.reduce((t, c) => t + c.length / 2, 0);
  }
  return { en, boy, cizgiler, nokta };
}

const ayniKisi = (a, b) => metin(a).toLocaleLowerCase('tr-TR') === metin(b).toLocaleLowerCase('tr-TR');

/**
 * İmza ön denetimi ve gövdesi. `girdi`: `{rol, imzalayan, cizim, imtina?, imtinaNotu?}`.
 * - Rol, tutanağın eksik rollerinden biri olmalı (aynı rol iki kez imzalanmaz).
 * - Aynı kişi iki rolde imzalayamaz (düzenleyen kendi tutanağına tanık olamaz).
 * - İmtina (imzadan kaçınma) yalnız şablonun `imtinaKaydi` verdiği rolde yazılır; çizim yerine 5-300 karakter not ister.
 * - Henüz sunucu kimliği olmayan tutanağa imza `istemci_no` (giden `no`) ile bağlanır.
 * @returns {{hatalar: string[], govde?: object}}
 */
export function imzaHazirla(tutanak, girdi, sablon, zamanMs = Date.now()) {
  const hatalar = [];
  if (!tutanak) return { hatalar: ['tutanak seçilmedi'] };
  if (tutanak.tur !== 'tutanak') return { hatalar: ['fotoğraf kaydı imzalanmaz; önce sunucuda çözümlenip tutanağa dönüşmeli'] };
  if (tutanak.id === undefined && !tutanak.istemci_no) return { hatalar: ['tutanak seçilmedi'] };
  if (tutanak.durum === 'iptal') hatalar.push('iptal edilmiş tutanak imzalanmaz');
  if (tutanak.reddedildi) hatalar.push('sunucunun reddettiği tutanak imzalanmaz');
  const g = girdi || {};
  const rol = metin(g.rol);
  if (rol === '') hatalar.push('imza rolü seçilmedi');
  else if (!sablonRolleri(sablon).includes(rol)) hatalar.push(`bu tutanakta "${rol}" rolü yok`);
  else if (tutanak.imzalar.some((i) => i.rol === rol)) hatalar.push(`"${rol}" imzası zaten atılmış`);
  const imzalayan = metin(g.imzalayan).replace(/\s+/g, ' ');
  if (imzalayan.length < 3 || imzalayan.length > 80) hatalar.push('imzalayanın adı soyadı yazılmalı (3-80 karakter)');
  else if (tutanak.imzalar.some((i) => ayniKisi(i.imzalayan, imzalayan))) hatalar.push('aynı kişi iki rolde imzalayamaz');
  const imtina = g.imtina === true;
  let cizim = null;
  let imtinaNotu = '';
  if (imtina) {
    const izinli = sablon && sablon.imzalar.some((i) => i.rol === rol && i.imtinaKaydi);
    if (!izinli) hatalar.push('bu rol için imtina (imzadan kaçınma) kaydı tutulmaz');
    imtinaNotu = metin(g.imtinaNotu);
    if (imtinaNotu.length < 5 || imtinaNotu.length > 300) hatalar.push('imtina notu 5-300 karakter olmalı');
  } else {
    cizim = cizimDuzelt(g.cizim);
    if (!cizim) hatalar.push('imza çizilmedi (kutunun içine imzanızı atın)');
  }
  if (hatalar.length) return { hatalar };
  const govde = { sozlesme: SOZLESME.imza, rol, imzalayan, zaman: new Date(zamanMs).toISOString() };
  if (tutanak.id !== undefined) govde.tutanak_id = tutanak.id;
  else govde.istemci_no = tutanak.istemci_no;
  if (tutanak.sha256) govde.tutanak_sha256 = tutanak.sha256;
  if (imtina) Object.assign(govde, { imtina: true, imtina_notu: imtinaNotu });
  else govde.cizim = { en: cizim.en, boy: cizim.boy, cizgiler: cizim.cizgiler };
  return { hatalar, govde };
}
