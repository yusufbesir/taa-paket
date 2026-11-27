// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W0-kabuk 2026-11-27]
// Kabuğun DOM'a dokunmayan saf işlevleri (Node'da test edilir: web/test/kabuk.test.js).

export const YETKI_CEVRIMDISI_GUN = 7; // S3 kuralı: yetki belgesi 7 gün çevrimdışı geçerli
const GUN_MS = 24 * 60 * 60 * 1000;

/** `#/kasa/defter/3` -> {ad: 'kasa', alt: ['defter', '3']}; boş ya da geçersiz -> {ad: null, alt: []}. */
export function rotaCoz(hash) {
  const ham = String(hash ?? '').replace(/^#/, '').split('?')[0];
  const parcalar = ham.split('/').filter((p) => p !== '');
  const ad = parcalar[0];
  if (!ad || !/^[a-z0-9-]{1,64}$/.test(ad)) return { ad: null, alt: [] };
  return { ad, alt: parcalar.slice(1) };
}

/**
 * moduller.json içeriğini doğrular; bozuk girişleri atar, aynı adı bir kez alır.
 * Giriş: `{ad, baslik, izin?, aciklama?, dosyalar?}`.
 * `dosyalar`: modülün çevrimdışı çalışması için gereken dosyalar, `web/` köküne göre
 * (ör. "moduller/kasa/mantik.js", "moduller/ortak/dom.js"); web/moduller/LISTE.json ile aynı biçim.
 */
export function modulListesi(json) {
  const ham = Array.isArray(json) ? json : json?.moduller;
  if (!Array.isArray(ham)) return [];
  const gorulen = new Set();
  const liste = [];
  for (const m of ham) {
    if (!m || typeof m.ad !== 'string' || !/^[a-z0-9-]{1,64}$/.test(m.ad) || gorulen.has(m.ad)) continue;
    gorulen.add(m.ad);
    liste.push({
      ad: m.ad,
      baslik: typeof m.baslik === 'string' && m.baslik.trim() !== '' ? m.baslik.trim() : m.ad,
      izin: typeof m.izin === 'string' && m.izin !== '' ? m.izin : null,
      aciklama: typeof m.aciklama === 'string' ? m.aciklama : '',
      dosyalar: Array.isArray(m.dosyalar) ? m.dosyalar.filter(dosyaYoluGecerli) : [],
    });
  }
  return liste;
}

/** `web/` köküne göre, yalnız `moduller/` altında kalan dosya yolu mu (`..`, kök, şema, sorgu yok). */
export function dosyaYoluGecerli(yol) {
  return (
    typeof yol === 'string' &&
    /^moduller\/[A-Za-z0-9._/-]{1,200}$/.test(yol) &&
    !yol.split('/').some((p) => p === '' || p === '.' || p === '..')
  );
}

/**
 * İzin denetimi (yalnız arayüzü süzer; asıl denetim sunucudadır).
 * İzin, kullanıcının izin/rol listesinde aynen varsa, `*` varsa ya da `kasa.*` gibi bir önek eşleşirse true.
 */
export function yetkiVar(profil, izin) {
  if (!profil || typeof izin !== 'string' || izin === '') return false;
  const liste = [...(profil.izinler ?? []), ...(profil.roller ?? [])];
  return liste.some(
    (i) => i === '*' || i === izin || (typeof i === 'string' && i.endsWith('.*') && izin.startsWith(i.slice(0, -1))),
  );
}

/** `POST /v1/giris` yanıtını (T7: `{oturum, sube_id, roller[]}`) profile çevirir. Anahtar profile GİRMEZ. */
export function girisYanitiCoz(yanit, kullanici, simdi = Date.now()) {
  if (!yanit || typeof yanit.oturum !== 'string' || yanit.oturum === '') {
    throw new Error('Sunucu yanıtında oturum yok.');
  }
  const sube = yanit.sube_id ?? yanit.sube;
  if (sube === undefined || sube === null || String(sube) === '') {
    throw new Error('Sunucu yanıtında şube yok.');
  }
  const metinler = (d) => (Array.isArray(d) ? d.filter((x) => typeof x === 'string') : []);
  return {
    oturum: yanit.oturum,
    profil: {
      kullanici: String(kullanici),
      sube: String(sube),
      roller: metinler(yanit.roller),
      izinler: metinler(yanit.izinler),
      zaman: simdi,
    },
  };
}

/** Yerel veritabanı adına girecek parçayı güvenli hale getirir (çekirdek tire kabul etmez). */
export function dbParcasi(deger) {
  // Geri dönüşlü kodlama: izinli olmayan her karakter `_xx` (onaltılık UTF-8). `_` da kodlanır ki
  // "a-b" ile "a_2db" gibi iki ayrı kimlik aynı ada düşmesin.
  let s = '';
  for (const bayt of new TextEncoder().encode(String(deger))) {
    const k = String.fromCharCode(bayt);
    s += /[A-Za-z0-9.]/.test(k) ? k : `_${bayt.toString(16).padStart(2, '0')}`;
  }
  if (s === '' || s.length > 48 || s === '.' || s === '..') {
    throw new Error('Şube ya da kullanıcı kimliği yerel veritabanı adı için uygun değil.');
  }
  return s;
}

/** Profil çevrimdışı kullanım için hâlâ geçerli mi (son girişten bu yana 7 gün). */
export function profilGecerli(profil, simdi = Date.now()) {
  return Boolean(
    profil &&
      typeof profil.kullanici === 'string' &&
      typeof profil.sube === 'string' &&
      Number.isFinite(profil.zaman) &&
      simdi - profil.zaman < YETKI_CEVRIMDISI_GUN * GUN_MS &&
      simdi >= profil.zaman - GUN_MS, // saat geri alınmışsa bir güne kadar hoş gör
  );
}

/**
 * Çevrimdışı göstergesi.
 * @returns {{metin: string, tur: 'tamam'|'uyari'|'hata'}}
 */
export function gostergeMetni({ cevrimici, bekleyen = 0, reddedilen = 0, oturumVar = true, kalici = true }) {
  const parcalar = [];
  if (!cevrimici) parcalar.push('Çevrimdışı');
  if (bekleyen > 0) parcalar.push(`Gönderilmeyen ${bekleyen} kayıt`);
  if (reddedilen > 0) parcalar.push(`Reddedilen ${reddedilen} kayıt`);
  if (cevrimici && bekleyen > 0 && !oturumVar) parcalar.push('göndermek için giriş yapın');
  if (!kalici) parcalar.push('bu tarayıcı kayıtları saklayamıyor');
  if (parcalar.length === 0) return { metin: 'Tüm kayıtlar gönderildi', tur: 'tamam' };
  const tur = reddedilen > 0 || !kalici ? 'hata' : 'uyari';
  return { metin: parcalar.join(' · '), tur };
}

/** Çıkış uyarısı metni; bekleyen yoksa `null`. */
export function cikisUyarisi(bekleyen) {
  if (!(bekleyen > 0)) return null;
  return `Gönderilmemiş ${bekleyen} kayıt var. Çıkmadan önce gönderin; göndermeden çıkarsanız kayıtlar bu cihazda kalır ve aynı kullanıcı yeniden giriş yapana kadar sunucuya ulaşmaz.`;
}

/** Service worker'ın önbelleğe alacağı modül dosyaları (kabuk klasörüne göre göreli). sw.js aynı kuralı uygular. */
export function modulDosyalari(liste) {
  const yollar = [];
  for (const m of liste) {
    yollar.push(`../moduller/${m.ad}/index.js`);
    for (const d of m.dosyalar ?? []) {
      if (dosyaYoluGecerli(d)) yollar.push(`../${d}`);
    }
  }
  return [...new Set(yollar)];
}
