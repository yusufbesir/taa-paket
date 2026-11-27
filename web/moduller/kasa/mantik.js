// Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27]
// TAA Kasa (web) saf kuralları: sayım, masraf, gün sonu karşılaştırması, çift doğrulama ön izlemesi, imza.
// Kaynak: crates/taa-kasa/src/gunsonu.rs ve tutanak.rs (kurallar oradan taşındı), docs/sozlesmeler/kasa.gunsonu.md,
// kasa.masraf.md. Buradaki denetim ön denetimdir; asıl karar sunucudadır (kişi, sıra numarası, tek gün sonu kuralı).
// DOM ve ağ yok. Tüm tutarlar kuruş, tam sayı.

import { tarihGecerli, TUTAR_EN_COK, paraYaz } from '../ortak/bicim.js';

export const PROGRAM = 'kasa';
export const SOZLESME_GUNSONU = 'kasa.gunsonu@1';
export const SOZLESME_MASRAF = 'kasa.masraf@1';
/** Sunucu uçları: docs/WEB-ESITLE-SOZLESMESI.md (taa-sunucu'da uygulanacak). Adlar varsayımdır, tek yerden değişir. */
export const UC_GUNSONU = '/v1/kasa/gunsonu';
export const UC_MASRAF = '/v1/kasa/masraf';

/** İzin adları (baglam.yetki ile sorulur). Sunucudaki adlar kesinleşince yalnız burası değişir. */
export const IZIN = {
  sayim: 'kasa.sayim',
  masraf: 'kasa.masraf',
  gunsonu: 'kasa.gunsonu',
  imza: 'kasa.imza',
};

/** Önbellek anahtarları (salt okunur; sunucu deltasından dolar). */
export const ANAHTAR = {
  ayar: 'kasa/ayar',
  gun: (tarih, cekmece) => `kasa/gun/${tarih}/${cekmece}`,
  gunSonlari: 'kasa/gunsonlari',
};

export const DEGERLER = [20000, 10000, 5000, 2000, 1000, 500, 100, 50, 25, 10, 5, 1];

export const ODEME_TURLERI = [
  ['nakit', 'NAKİT TL'],
  ['ziraat_pos', 'ZİRAAT BANKASI POS:1'],
  ['iyzico_pos', 'İYZİCO POS:2'],
  ['havale', 'BANKAYA HAVALE (firma hesabı)'],
  ['iyzico_odemesi', 'İYZİCO ÖDEMESİ'],
  ['kredi_ceki', 'KREDİ ÇEKİ'],
  ['internet_degisimi', 'İNTERNET DEĞİŞİMİ'],
];
export const ESKI_EFATURA = 'efatura_pos';
export const KASA_ACIGI = 'kasa_acigi';
export const BILGI_ALANLARI = [
  ['kredi_karti_fark', 'KREDİ KARTI +/-'],
  ['kasa_fazlasi', 'KASA FAZLASI'],
  ['havale_fark', 'HAVALE +/-'],
  ['degisim', 'DEĞİŞİM'],
  ['iade', 'İADE'],
];
/** Elle girilen bilgi satırları (kasa fazlası sayımdan hesaplanır, girilmez). */
export const BILGI_GIRILEN = BILGI_ALANLARI.filter(([k]) => k !== 'kasa_fazlasi');
export const ACIKLAMALI_ALANLAR = ['havale_fark', 'kredi_karti_fark'];
export const DOVIZLER = [
  ['USD', 'Dolar'],
  ['EUR', 'Euro'],
  ['GBP', 'Sterlin'],
  ['CHF', 'İsviçre Frangı'],
];
export const KAPANIS_LISTESI = [
  'Z raporu alındı',
  'Z raporu, POS ve satış sistemi karşılaştırıldı',
  'Fiili sayım tutanağı hazırlandı (imzalar liste bitince alınır)',
  'Kasa kilitlendi',
  'Kamera kaydı açık',
  'Masraf fişleri ayrıldı',
  'Ertesi gün bozukluğu hazırlandı',
];
export const IMZA_ROLLERI = [
  ['kasa_sorumlusu', 'Kasa Sorumlusu'],
  ['magaza_sorumlusu', 'Mağaza Sorumlusu / Vekili'],
];
export const IKINCI_IMZA_ATMADI = 'ikinci imza atmadı';
export const GEREKCE_EN_AZ = 5;
export const GEREKCE_EN_COK = 300;
export const MASRAF_TURLERI = [
  'Personel Yemek Masrafı',
  'Personel İçecek Masrafı',
  'Mağaza Gideri',
  'Temizlik',
  'Kargo',
  'Ulaşım',
  'Bakım-Onarım',
  'Kırtasiye',
  'Müşteri İadesi',
  'Diğer',
];
export const MASRAF_TURU_EN_UZUN = 60;
export const SAYIM_TURLERI = [
  ['acilis', 'Açılış sayımı'],
  ['ara', 'Ara sayım'],
  ['devir', 'Vardiya devri'],
];
const Z_FORMULU = { degisim_eklenir: true, internet_degisimi_dusulur: true, iade_eklenir: true, kredi_ceki_dusulur: true };

const tamSayi = (n) => Number.isSafeInteger(n);
const uzunluk = (s) => [...String(s)].length;
const kontrolKarakteri = (s) => /[\u0000-\u001f\u007f]/.test(s);
const ayniKisi = (a, b) =>
  String(a ?? '').trim().toLocaleLowerCase('tr') === String(b ?? '').trim().toLocaleLowerCase('tr') &&
  String(a ?? '').trim() !== '';

/**
 * Önbellekten gelen mağaza ayarını denetler ve varsayılanlarla tamamlar. Çekmece tanımı yoksa `null`:
 * ayar inmeden kayıt alınmaz (tanımsız çekmeceye yazılan kayıt sunucuda reddedilir).
 */
export function ayarDuzelt(ham) {
  if (!ham || typeof ham !== 'object' || !Array.isArray(ham.cekmeceler) || ham.cekmeceler.length === 0) return null;
  const cekmeceler = [];
  for (const c of ham.cekmeceler) {
    if (!c || typeof c.id !== 'string' || c.id === '') return null;
    cekmeceler.push({
      id: c.id,
      ad: String(c.ad || c.id),
      noktalar: (Array.isArray(c.noktalar) ? c.noktalar : []).map((n) => ({
        id: String(n.id),
        ad: String(n.ad || n.id),
        poslar: (Array.isArray(n.poslar) ? n.poslar : []).map((p) => ({
          id: String(p.id),
          ad: String(p.ad || p.id),
          odeme_turu: String(p.odeme_turu || ''),
        })),
      })),
    });
  }
  const sayi = (v, varsayilan) => (tamSayi(v) && v >= 0 ? v : varsayilan);
  const turler = Array.isArray(ham.masraf_turleri) ? ham.masraf_turleri.filter((t) => typeof t === 'string' && t.trim()) : [];
  return {
    sube: String(ham.sube || ''),
    cekmeceler,
    masraf_merkez_esigi_kurus: sayi(ham.masraf_merkez_esigi_kurus, 200_000),
    fark_toleransi_kurus: sayi(ham.fark_toleransi_kurus, 0),
    masraf_turleri: turler.length ? turler : [...MASRAF_TURLERI],
    gerekce_sablonlari: Array.isArray(ham.gerekce_sablonlari) ? ham.gerekce_sablonlari.map(String) : [],
    z_formulu: { ...Z_FORMULU, ...(ham.z_formulu && typeof ham.z_formulu === 'object' ? ham.z_formulu : {}) },
  };
}

export function cekmeceBul(ayar, id) {
  return ayar.cekmeceler.find((c) => c.id === id) || null;
}

function tutarHatasi(ad, t) {
  return tamSayi(t) && t >= 0 && t <= TUTAR_EN_COK ? null : `${ad}: tutar 0 ile 10 milyar TL arasında olmalı`;
}

/** Kupür adetlerinden toplam. `{deger}` ya da `{hata}` döner. Sıfır adetli kupürler kayda girmez. */
export function sayimToplam(adetler) {
  let t = 0;
  const temiz = {};
  for (const [d, a] of Object.entries(adetler || {})) {
    const deger = Number(d);
    if (!DEGERLER.includes(deger)) return { hata: `geçersiz para değeri: ${d} kuruş` };
    if (!tamSayi(a) || a < 0 || a > 4_294_967_295) return { hata: `${paraYaz(deger)} için adet geçersiz` };
    if (a === 0) continue;
    t += deger * a;
    if (!tamSayi(t)) return { hata: 'taşma' };
    temiz[String(deger)] = a;
  }
  return { deger: t, adetler: temiz };
}

/** Dövizin TL karşılığı (kuruş), yarım yukarı yuvarlanır: tutar × kur_onbinde / 10000. */
export function dovizTl(d) {
  return Number((BigInt(d.tutar) * BigInt(d.kur_onbinde) + 5000n) / 10000n);
}

/** `ciro` alanında geçerli anahtar mı; dönen değer "eksi olabilir mi". Bilinmeyen anahtar `null`. */
export function ciroAnahtari(k) {
  if (k === 'kredi_ceki') return true;
  if (ODEME_TURLERI.some(([a]) => a === k) || k === KASA_ACIGI) return false;
  if (BILGI_ALANLARI.some(([a]) => a === k)) return true;
  return null;
}

/** Şablondaki genel toplam: ödeme satırları + döviz TL karşılığı + masraflar. Bilgi satırları girmez. */
export function ciroToplami(ciro, doviz, masrafKurus) {
  const c = (k) => ciro[k] || 0;
  return (
    ODEME_TURLERI.reduce((t, [k]) => t + c(k), 0) +
    c(ESKI_EFATURA) +
    (doviz || []).reduce((t, d) => t + dovizTl(d), 0) +
    masrafKurus
  );
}

/** Z raporlarında beklenen toplam: genel toplam + değişim − internet değişimi + iade − kredi çeki (ayarlı). */
export function zBeklenen(ciroToplam, ciro, f = Z_FORMULU) {
  const c = (k) => ciro[k] || 0;
  const e = (acik, k) => (acik ? c(k) : 0);
  return (
    ciroToplam +
    e(f.degisim_eklenir, 'degisim') -
    e(f.internet_degisimi_dusulur, 'internet_degisimi') +
    e(f.iade_eklenir, 'iade') -
    e(f.kredi_ceki_dusulur, 'kredi_ceki')
  );
}

/** POS gün sonlarını ödeme türüne göre toplar: `{odeme_turu: {ad, toplam}}`. */
export function posToplamlari(ayar, girdi) {
  const sonuc = new Map();
  const cek = cekmeceBul(ayar, girdi.cekmece);
  for (const n of cek ? cek.noktalar : []) {
    const ng = (girdi.noktalar || []).find((x) => x.nokta === n.id);
    for (const p of n.poslar) {
      const t = (ng && ng.pos && ng.pos[p.id]) || 0;
      if (!sonuc.has(p.odeme_turu)) sonuc.set(p.odeme_turu, { ad: p.ad, toplam: 0 });
      sonuc.get(p.odeme_turu).toplam += t;
    }
  }
  return sonuc;
}

/**
 * Gün sonu karşılaştırması (gunsonu.rs `karsilastir`). `acilisKurus`: o günün açılış sayımı; `masrafKurus`:
 * çekmeceden çıkan (reddedilmemiş, iptal edilmemiş) masraflar. Çekmecede beklenen = açılış + NAKİT TL.
 */
export function karsilastir(ayar, girdi, nakitSayilan, acilisKurus, masrafKurus) {
  const ciro = girdi.ciro || {};
  const cir = (k) => ciro[k] || 0;
  const ciroToplam = ciroToplami(ciro, girdi.doviz, masrafKurus);
  const z = (girdi.noktalar || []).reduce((t, n) => t + (n.z_toplam_kurus || 0), 0);
  const satirlar = [
    { ad: 'Nakit: sayılan ↔ beklenen (açılış + NAKİT TL)', beklenen_kurus: acilisKurus + cir('nakit'), bulunan_kurus: nakitSayilan },
    {
      ad: 'Z raporu toplamı ↔ genel toplam + değişim (internet hariç) + iade',
      beklenen_kurus: z,
      bulunan_kurus: zBeklenen(ciroToplam, ciro, ayar.z_formulu),
    },
  ];
  const pos = [...posToplamlari(ayar, girdi).entries()].sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
  for (const [tur, { ad, toplam }] of pos) {
    const turAd = (ODEME_TURLERI.find(([k]) => k === tur) || [tur, tur])[1];
    satirlar.push({ ad: `${ad} gün sonu ↔ ${turAd} cirosu`, beklenen_kurus: toplam, bulunan_kurus: cir(tur) });
  }
  satirlar.push({
    ad: 'Satış sistemi (Nebim) toplamı ↔ genel toplam (şablon)',
    beklenen_kurus: girdi.nebim_toplam_kurus || 0,
    bulunan_kurus: ciroToplam,
  });
  for (const s of satirlar) s.fark_kurus = s.bulunan_kurus - s.beklenen_kurus;
  const dis = (s) => Math.abs(s.fark_kurus) > ayar.fark_toleransi_kurus;
  return {
    satirlar,
    ciro_toplam_kurus: ciroToplam,
    fark_var: satirlar.some(dis),
    kasa_ciro_uyusmuyor: satirlar.slice(0, 2).some(dis),
  };
}

function ortakDenetim(ayar, tarih, cekmece) {
  const h = [];
  if (!tarihGecerli(tarih)) h.push('tarih YYYY-AA-GG olmalı');
  if (!cekmeceBul(ayar, cekmece)) h.push(`tanımsız çekmece: ${cekmece || '(boş)'}`);
  return h;
}

/**
 * Sayım (açılış / ara / devir) kaydını doğrular ve giden kutusu gövdesini hazırlar.
 * `ek.beklenenKurus`: biliniyorsa beklenen nakit (açılışta önceki günün devredeni; ara/devirde açılış − masraf);
 * bilinmiyorsa `null` (fark hesaplanmaz). `ek.girenAd`: biliniyorsa işlemi giren kişi.
 * Dönüş: `{hatalar: [...], govde}`; hata varsa `govde` yoktur.
 */
export function sayimHazirla(ayar, tur, g, ek = {}) {
  const hatalar = ortakDenetim(ayar, g.tarih, g.cekmece);
  if (!SAYIM_TURLERI.some(([k]) => k === tur)) hatalar.push(`bilinmeyen sayım türü: ${tur}`);
  const ikinci = String(g.ikinci_kisi ?? '').trim();
  if (tur === 'ara' || tur === 'devir') {
    if (!ikinci) hatalar.push(tur === 'devir' ? 'devralan kişi gerekli' : 'ara sayım iki kişiyle yapılır: ikinci kişi gerekli');
    else if (ek.girenAd && ayniKisi(ikinci, ek.girenAd)) hatalar.push('ikinci kişi işlemi girenden farklı olmalı');
  }
  const toplam = sayimToplam(g.adetler);
  if (toplam.hata) hatalar.push(toplam.hata);
  const beklenen = tamSayi(ek.beklenenKurus) ? ek.beklenenKurus : null;
  const not = String(g.not ?? '').trim();
  let fark = null;
  if (!toplam.hata && beklenen !== null) {
    fark = toplam.deger - beklenen;
    if (Math.abs(fark) > ayar.fark_toleransi_kurus && !not) hatalar.push(`${paraYaz(fark)} fark var; açıklama notu yazın`);
  }
  if (uzunluk(not) > 300) hatalar.push('not en çok 300 karakter');
  if (hatalar.length) return { hatalar };
  return {
    hatalar,
    govde: {
      sozlesme: SOZLESME_GUNSONU,
      tur: 'sayim',
      v: {
        tur,
        tarih: g.tarih,
        cekmece: g.cekmece,
        ikinci_kisi: ikinci || null,
        adetler: toplam.adetler,
        sayilan_kurus: toplam.deger,
        beklenen_kurus: beklenen,
        fark_kurus: fark,
        not,
      },
    },
  };
}

/** Masraf kaydını doğrular (gunsonu.rs `masraf_ekle`) ve gövdeyi hazırlar. */
export function masrafHazirla(ayar, g) {
  const hatalar = ortakDenetim(ayar, g.tarih, g.cekmece);
  const th = tutarHatasi('masraf', g.tutar_kurus);
  if (th) hatalar.push(th);
  else if (g.tutar_kurus === 0) hatalar.push('masraf tutarı girin: tutarı olmayan kalem kaydedilmez (toplam kalemlerden hesaplanır)');
  const tur = String(g.tur ?? '').trim();
  if (!tur) hatalar.push('masraf türü gerekli (listeden seçin ya da yazın)');
  else if (uzunluk(tur) > MASRAF_TURU_EN_UZUN || kontrolKarakteri(tur)) hatalar.push(`masraf türü en çok ${MASRAF_TURU_EN_UZUN} karakter, tek satır olmalı`);
  const aciklama = String(g.aciklama ?? '').trim();
  if (!aciklama) hatalar.push('masraf açıklaması gerekli');
  if (hatalar.length) return { hatalar };
  return {
    hatalar,
    govde: {
      sozlesme: SOZLESME_MASRAF,
      tur: 'masraf',
      v: {
        tarih: g.tarih,
        cekmece: g.cekmece,
        tur,
        tutar_kurus: g.tutar_kurus,
        aciklama,
        not: [...String(g.not ?? '').trim()].slice(0, 200).join(''),
        onay_duzeyi: g.tutar_kurus >= ayar.masraf_merkez_esigi_kurus ? 'merkez' : 'mudur',
      },
    },
  };
}

/**
 * Gün sonu girdisini doğrular (gunsonu.rs `gun_sonu_hazirla`), karşılaştırmayı hesaplar, kasa fazlası/açığını
 * sayım farkından yazar ve gövdeyi hazırlar. `ek.acilisKurus` bilinmiyorsa 0 sayılır (Rust ile aynı) ve
 * `acilis_biliniyor: false` yazılır. Dönüş: `{hatalar, karsilastirma, govde}`.
 */
export function gunSonuHazirla(ayar, g, ek = {}) {
  const hatalar = ortakDenetim(ayar, g.tarih, g.cekmece);
  const cek = cekmeceBul(ayar, g.cekmece);
  const noktalar = [];
  for (const n of g.noktalar || []) {
    const tanim = cek && cek.noktalar.find((x) => x.id === n.nokta);
    if (cek && !tanim) {
      hatalar.push(`bu çekmeceye bağlı olmayan kasa noktası: ${n.nokta}`);
      continue;
    }
    const zh = tutarHatasi('Z raporu', n.z_toplam_kurus);
    if (zh) hatalar.push(zh);
    for (const [p, t] of Object.entries(n.pos || {})) {
      if (tanim && !tanim.poslar.some((x) => x.id === p)) hatalar.push(`tanımsız POS: ${p}`);
      const ph = tutarHatasi('POS gün sonu', t);
      if (ph) hatalar.push(ph);
    }
    noktalar.push({ nokta: n.nokta, z_toplam_kurus: n.z_toplam_kurus, pos: { ...(n.pos || {}) } });
  }
  const ciro = {};
  for (const [k, t] of Object.entries(g.ciro || {})) {
    if (k === ESKI_EFATURA) {
      if (t !== 0) hatalar.push('E-FATURA POS kaldırıldı (iki POS terminali var): tutarı İYZİCO POS\'a yazın');
      continue;
    }
    const eksiOlur = ciroAnahtari(k);
    if (eksiOlur === null) {
      hatalar.push(`bilinmeyen ciro alanı: ${k}`);
      continue;
    }
    const ch = tamSayi(t) ? tutarHatasi(k, eksiOlur ? Math.abs(t) : t) : `${k}: tutar geçersiz`;
    if (ch) hatalar.push(ch);
    ciro[k] = t;
  }
  const doviz = [];
  for (const d of g.doviz || []) {
    if (!DOVIZLER.some(([k]) => k === d.kod)) {
      hatalar.push(`bilinmeyen döviz: ${d.kod}`);
      continue;
    }
    const dh = tutarHatasi('döviz', d.tutar);
    if (dh) {
      hatalar.push(dh);
      continue;
    }
    if (d.tutar > 0 && !(tamSayi(d.kur_onbinde) && d.kur_onbinde >= 1 && d.kur_onbinde <= 100_000_000)) {
      hatalar.push(`${d.kod} için günlük kur girin`);
      continue;
    }
    if (d.tutar > 0) doviz.push({ kod: d.kod, tutar: d.tutar, kur_onbinde: d.kur_onbinde });
  }
  const aciklamalar = {};
  for (const [k, v] of Object.entries(g.aciklamalar || {})) {
    if (!ACIKLAMALI_ALANLAR.includes(k)) {
      hatalar.push(`açıklama yazılamayan alan: ${k}`);
      continue;
    }
    const m = String(v ?? '').trim();
    if (uzunluk(m) > 300) hatalar.push('açıklama en çok 300 karakter');
    else if (m) aciklamalar[k] = m;
  }
  for (const k of ACIKLAMALI_ALANLAR) {
    if ((ciro[k] || 0) !== 0 && !aciklamalar[k]) {
      const ad = BILGI_ALANLARI.find(([a]) => a === k)[1];
      hatalar.push(`${ad} için açıklama yazın (farkın nedeni)`);
    }
  }
  const nh = tutarHatasi('Nebim toplamı', g.nebim_toplam_kurus);
  if (nh) hatalar.push(nh);
  const dvh = tutarHatasi('devreden nakit', g.devreden_kurus);
  if (dvh) hatalar.push(dvh);
  const nakit = sayimToplam(g.nakit_adetler);
  if (nakit.hata) hatalar.push(nakit.hata);
  else if (!dvh && g.devreden_kurus > nakit.deger) hatalar.push('ertesi güne devreden nakit sayılan nakitten büyük olamaz');
  if (hatalar.length) return { hatalar };

  const kontrol = KAPANIS_LISTESI.map((_, i) => Boolean((g.kontrol || [])[i]));
  const acilisBiliniyor = tamSayi(ek.acilisKurus);
  const acilis = acilisBiliniyor ? ek.acilisKurus : 0;
  const masraf = tamSayi(ek.masrafKurus) ? ek.masrafKurus : 0;
  const temiz = { ...g, ciro, doviz, noktalar };
  const k = karsilastir(ayar, temiz, nakit.deger, acilis, masraf);
  // Kasa fazlası / açığı elle girilmez: nakit sayım farkından yazılır.
  delete ciro.kasa_fazlasi;
  delete ciro[KASA_ACIGI];
  const nf = k.satirlar[0].fark_kurus;
  if (nf > 0) ciro.kasa_fazlasi = nf;
  else if (nf < 0) ciro[KASA_ACIGI] = -nf;
  const not = String(g.not ?? '').trim();
  if (k.fark_var && !not) return { hatalar: ['karşılaştırmada fark var; açıklama notu yazın'], karsilastirma: k };
  return {
    hatalar,
    karsilastirma: k,
    govde: {
      sozlesme: SOZLESME_GUNSONU,
      tur: 'gun_sonu',
      v: {
        tarih: g.tarih,
        sube: ayar.sube,
        cekmece: g.cekmece,
        nakit_adetler: nakit.adetler,
        nakit_sayilan_kurus: nakit.deger,
        noktalar,
        nebim_toplam_kurus: g.nebim_toplam_kurus,
        ciro,
        doviz,
        devreden_kurus: g.devreden_kurus,
        kontrol,
        not,
        aciklamalar,
        karsilastirma: k,
        // İstemcinin karşılaştırmada kullandığı değerler; sunucu kendi kayıtlarıyla yeniden hesaplar.
        acilis_kurus: acilis,
        acilis_biliniyor: acilisBiliniyor,
        masraf_kurus: masraf,
      },
    },
  };
}

const KALEMLER = [
  ['nakit_tl', 'Nakit (TL)'],
  ['ziraat_pos', 'Ziraat Bankası POS'],
  ['iyzico_fiziksel_pos', 'İyzico Fiziksel POS'],
  ['iyzico_sanal_pos', 'İyzico Sanal POS'],
  ['havale_eft', 'Havale / EFT'],
  ['kredi_ceki', 'Kredi Çeki'],
  ['eticaret_degisimi', 'E-ticaret Değişimi'],
  ['diger', 'Diğer'],
];

/**
 * Kasa Çift Doğrulama Tutanağı ön izlemesi: 8 sabit kalem, fark = fiili − ERP (tutanak.rs eşleme kuralı).
 * ERP sütunu gün sonundaki ciro satırlarıdır (Nebim kasa Excel'i içe aktarma web'de yok). Resmî tutanak
 * sunucuda/TAA Kasa'da üretilir; bu yalnız ekranda göstermek içindir.
 */
export function ciftDogrulama(ayar, v) {
  const c = (k) => v.ciro[k] || 0;
  const pos = posToplamlari(ayar, v);
  const posT = (tur) => (pos.get(tur) || { toplam: 0 }).toplam;
  const dovizTop = (v.doviz || []).reduce((t, d) => t + dovizTl(d), 0);
  const krediErp = c('kredi_ceki') + c('degisim') - c('internet_degisimi');
  const tutar = {
    nakit_tl: [c('nakit'), v.nakit_sayilan_kurus - (v.acilis_kurus || 0)],
    ziraat_pos: [c('ziraat_pos'), posT('ziraat_pos')],
    iyzico_fiziksel_pos: [c('iyzico_pos') + c(ESKI_EFATURA), posT('iyzico_pos')],
    iyzico_sanal_pos: [c('iyzico_odemesi'), c('iyzico_odemesi')],
    havale_eft: [c('havale'), c('havale') + c('havale_fark')],
    kredi_ceki: [krediErp, krediErp],
    eticaret_degisimi: [c('internet_degisimi'), c('internet_degisimi')],
    diger: [dovizTop, dovizTop],
  };
  const kalemler = KALEMLER.map(([kod, ad]) => {
    const [erp, fiili] = tutar[kod];
    const fark = fiili - erp;
    return { kod, ad, erp_kurus: erp, fiili_kurus: fiili, fark_kurus: fark, gerekce: fark !== 0 ? v.not || '' : '' };
  });
  const farkli = kalemler.filter((k) => k.fark_kurus !== 0);
  return {
    kalemler,
    toplam_fark_kurus: kalemler.reduce((t, k) => t + k.fark_kurus, 0),
    fark_tutar_kurus: kalemler.reduce((t, k) => t + Math.abs(k.fark_kurus), 0),
    fark_kalem: farkli.length,
    sonuc: farkli.length ? 'Fark tespit edilmiştir.' : 'Fark yoktur.',
  };
}

/**
 * Gün sonu imzası ön denetimi (çift doğrulama). `gun`: `{id?, istemci_no?, kisi?, kontrol, imzalar:[{sira,kisi}], gonderildi?}`.
 * `imzalayanAd` biliniyorsa "aynı kişi iki kez imzalayamaz" ve "ikinci imzayı giren atamaz" burada da denetlenir;
 * bilinmiyorsa bu iki kural yalnız sunucuda denetlenir.
 */
export function imzaHazirla(gun, imzalayanAd) {
  const hatalar = [];
  const imzalar = Array.isArray(gun.imzalar) ? gun.imzalar : [];
  if (!gun.id && !gun.istemci_no) hatalar.push('gün sonu kaydı seçilmedi');
  if (gun.gonderildi) hatalar.push('gönderilmiş güne imza eklenmez');
  if (!Array.isArray(gun.kontrol) || gun.kontrol.length < KAPANIS_LISTESI.length || !gun.kontrol.every(Boolean)) {
    hatalar.push('imza isteği kapanış listesi tamamlanınca açılır (7 madde)');
  }
  if (imzalar.length >= 2) hatalar.push('iki imza da atılmış');
  const sira = imzalar.length + 1;
  // Giden kutusu kullanıcıya özeldir (veritabanı "taa-<şube>-<kullanıcı>"): bu oturumda bekleyen imza ya da
  // gün sonu, aynı kişinin işidir.
  if (imzalar.some((i) => i.bekliyor)) hatalar.push('bu oturumdan imza atıldı; aynı kişi iki kez imzalayamaz');
  else if (sira === 2 && gun.bekliyor) hatalar.push('ikinci imzayı gün sonunu giren atamaz');
  else if (imzalayanAd) {
    if (imzalar.some((i) => ayniKisi(i.kisi, imzalayanAd))) hatalar.push('aynı kişi iki kez imzalayamaz');
    else if (sira === 2 && ayniKisi(gun.kisi, imzalayanAd)) hatalar.push('ikinci imzayı gün sonunu giren atamaz');
  }
  if (hatalar.length) return { hatalar };
  const v = { sira, rol: IMZA_ROLLERI[sira - 1][0], kanal: 'ekran' };
  if (gun.id) v.id = gun.id;
  else v.istemci_no = gun.istemci_no;
  return { hatalar, govde: { sozlesme: SOZLESME_GUNSONU, tur: 'imza', v } };
}

/** İkinci imza atılmayacaksa gerekçe (5–300 karakter); yalnız tek imzalı, gönderilmemiş güne yazılır. */
export function gerekceHazirla(gun, gerekce) {
  const hatalar = [];
  const imzalar = Array.isArray(gun.imzalar) ? gun.imzalar : [];
  const m = String(gerekce ?? '').trim();
  if (!gun.id && !gun.istemci_no) hatalar.push('gün sonu kaydı seçilmedi');
  if (gun.gonderildi) hatalar.push('gönderilmiş güne gerekçe yazılmaz');
  if (imzalar.length !== 1) hatalar.push('gerekçe yalnız tek imzalı güne yazılır');
  if (uzunluk(m) < GEREKCE_EN_AZ || uzunluk(m) > GEREKCE_EN_COK || kontrolKarakteri(m)) {
    hatalar.push(`gerekçe ${GEREKCE_EN_AZ}–${GEREKCE_EN_COK} karakter, tek satır olmalı`);
  }
  if (hatalar.length) return { hatalar };
  const v = { gerekce: m };
  if (gun.id) v.id = gun.id;
  else v.istemci_no = gun.istemci_no;
  return { hatalar, govde: { sozlesme: SOZLESME_GUNSONU, tur: 'gerekce', v } };
}

/**
 * Önbellekteki gün özeti ile giden kutusunda bekleyen kayıtları birleştirir.
 * `onbellekGun`: `{acilis_kurus?, masraf_kurus?, onceki_devreden_kurus?, gun_sonu_var?}` (sunucu görünümü).
 * `bekleyenler`: giden kutusunda `bekliyor` durumundaki kasa kayıtları (henüz sunucu görünümüne girmemiş).
 */
export function gunOzeti(onbellekGun, bekleyenler, tarih, cekmece) {
  const o = onbellekGun && typeof onbellekGun === 'object' ? onbellekGun : {};
  const ozet = {
    acilisKurus: tamSayi(o.acilis_kurus) ? o.acilis_kurus : null,
    masrafKurus: tamSayi(o.masraf_kurus) ? o.masraf_kurus : 0,
    oncekiDevredenKurus: tamSayi(o.onceki_devreden_kurus) ? o.onceki_devreden_kurus : null,
    gunSonuVar: Boolean(o.gun_sonu_var),
    acilisVar: tamSayi(o.acilis_kurus),
  };
  for (const k of bekleyenler || []) {
    const g = k.govde;
    if (!g || !g.v || g.v.tarih !== tarih || g.v.cekmece !== cekmece) continue;
    if (g.tur === 'masraf') ozet.masrafKurus += g.v.tutar_kurus;
    else if (g.tur === 'sayim' && g.v.tur === 'acilis') {
      ozet.acilisKurus = g.v.sayilan_kurus;
      ozet.acilisVar = true;
    } else if (g.tur === 'gun_sonu') ozet.gunSonuVar = true;
  }
  return ozet;
}

/**
 * İmza ekranı için gün sonu listesi: önbellekteki (sunucu) kayıtlar + giden kutusunda bekleyen gün sonları;
 * bekleyen imza ve gerekçeler ilgili kayda işlenir. `kayitlar`: giden kutusundaki tüm kasa kayıtları.
 */
export function imzaListesi(onbellekListe, kayitlar) {
  const liste = (Array.isArray(onbellekListe) ? onbellekListe : []).map((g) => ({
    id: g.id,
    tarih: g.tarih,
    cekmece: g.cekmece,
    kisi: g.kisi || '',
    kontrol: Array.isArray(g.kontrol) ? g.kontrol : [],
    imzalar: Array.isArray(g.imzalar) ? g.imzalar.map((i) => ({ sira: i.sira, kisi: i.kisi || '' })) : [],
    gerekce: g.gerekce || '',
    gonderildi: Boolean(g.gonderildi),
    bekliyor: false,
  }));
  const bekleyen = (kayitlar || []).filter((k) => k.durum === 'bekliyor' && k.govde);
  for (const k of bekleyen) {
    if (k.govde.tur !== 'gun_sonu') continue;
    const v = k.govde.v;
    liste.push({ istemci_no: k.no, tarih: v.tarih, cekmece: v.cekmece, kisi: '', kontrol: v.kontrol, imzalar: [], gerekce: '', gonderildi: false, bekliyor: true });
  }
  for (const k of bekleyen) {
    const { tur, v } = k.govde;
    if (tur !== 'imza' && tur !== 'gerekce') continue;
    const hedef = liste.find((g) => (v.id && g.id === v.id) || (v.istemci_no && g.istemci_no === v.istemci_no));
    if (!hedef) continue;
    if (tur === 'imza' && !hedef.imzalar.some((i) => i.sira === v.sira)) hedef.imzalar.push({ sira: v.sira, kisi: '', bekliyor: true });
    if (tur === 'gerekce') hedef.gerekce = v.gerekce;
  }
  return liste.sort((a, b) => (a.tarih < b.tarih ? 1 : a.tarih > b.tarih ? -1 : 0));
}
