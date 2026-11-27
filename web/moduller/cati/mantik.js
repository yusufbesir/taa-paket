// Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27]
// TAA Çatı (web) yönetici ekranı iskeleti: ekran listesi ve izin kapısı. Kaynak: docs/S3-CATI-PANEL.md
// ("Ekranlar (izne göre görünür)"). Kural: varsayılan reddet; izni olmayan ekran menüde de görünmez,
// verisi de okunmaz. Yönetim işlemleri (onay, iptal, PIN) henüz taşınmadı; iskelet yalnız okur.

export const PROGRAM = 'cati';

/** İzin adları S3-CATI-PANEL.md'deki adlardır; `yonetici` rol denetimidir (Raporlar yalnız yöneticiye). */
export const IZIN = {
  cihaz: 'cihaz.yonet',
  kullanici: 'kullanici.yonet',
  program: 'program.yonet',
  gunluk: 'gunluk.gor',
  yonetici: 'yonetici',
};

/**
 * Ekranlar: `izin` boşsa ekran, herhangi bir yönetim izni olan herkese açıktır (Genel bakış).
 * `anahtar`: önbellekten okunacak liste (yoksa ekran yalnız açıklama gösterir).
 */
export const EKRANLAR = [
  { kod: 'genel', ad: 'Genel bakış', izin: null, anahtar: null, aciklama: 'Bu cihazın giden kutusu ve yönetim ekranlarının özeti.' },
  { kod: 'cihazlar', ad: 'Cihazlar', izin: IZIN.cihaz, anahtar: 'cati/cihazlar', sutunlar: [['ad', 'Cihaz'], ['sube', 'Şube'], ['durum', 'Durum']], aciklama: 'Bekleyen, onaylı ve iptal edilen cihazlar.' },
  { kod: 'kullanicilar', ad: 'Kullanıcılar', izin: IZIN.kullanici, anahtar: 'cati/kullanicilar', sutunlar: [['ad', 'Ad'], ['rol', 'Rol'], ['sube', 'Şube'], ['durum', 'Durum']], aciklama: 'Kullanıcı listesi. Silme yok: hesap kapatılır ya da askıya alınır.' },
  { kod: 'subeler', ad: 'Şubeler', izin: IZIN.kullanici, anahtar: 'cati/subeler', sutunlar: [['kod', 'Kod'], ['ad', 'Ad']], aciklama: 'Şube listesi.' },
  { kod: 'yetkiler', ad: 'Program yetkileri', izin: IZIN.program, anahtar: 'cati/program-yetkileri', sutunlar: [['program', 'Program'], ['sube', 'Şube'], ['acik', 'Açık']], aciklama: 'Program × şube anahtarları.' },
  { kod: 'pinler', ad: 'Yönetici PIN\'leri', izin: IZIN.program, anahtar: 'cati/yonetici-pinleri', sutunlar: [['program', 'Program'], ['sube_id', 'Şube'], ['belirlendi', 'Belirlendi']], aciklama: 'PIN ve karması hiçbir zaman bu cihaza inmez; yalnız belirlenip belirlenmediği görünür.' },
  { kod: 'raporlar', ad: 'Raporlar', izin: IZIN.yonetici, anahtar: null, aciklama: 'Tüm TAA uygulamalarından rapor. Yalnız yöneticiler.' },
  { kod: 'gunluk', ad: 'Günlük', izin: IZIN.gunluk, anahtar: 'cati/gunluk', sutunlar: [['zaman', 'Zaman'], ['islem', 'İşlem'], ['kisi', 'Kişi']], aciklama: 'Olay günlüğü.' },
];

/** Modülün açılması için en az bir yönetim izni gerekir. */
export function yoneticiMi(yetki) {
  return Object.values(IZIN).some((i) => yetki(i) === true);
}

/** Kişinin görebileceği ekranlar. Yönetim izni yoksa boş liste (Genel bakış dahil hiçbir şey). */
export function gorunurEkranlar(yetki) {
  if (!yoneticiMi(yetki)) return [];
  return EKRANLAR.filter((e) => e.izin === null || yetki(e.izin) === true);
}

/** Giden kutusu sayıları: `{bekliyor, gonderildi, reddedildi}`. */
export function gidenSayilari(kayitlar) {
  const s = { bekliyor: 0, gonderildi: 0, reddedildi: 0 };
  for (const k of kayitlar || []) if (k && k.durum in s) s[k.durum] += 1;
  return s;
}

/** Önbellekten gelen listeyi tablo satırlarına çevirir (yalnız tanımlı sütunlar; başka alan ekrana çıkmaz). */
export function tabloSatirlari(ekran, ham) {
  const l = Array.isArray(ham) ? ham : ham && Array.isArray(ham.kayitlar) ? ham.kayitlar : [];
  return l
    .filter((k) => k && typeof k === 'object')
    .map((k) =>
      ekran.sutunlar.map(([alan]) => {
        const v = k[alan];
        if (v === true) return 'evet';
        if (v === false) return 'hayır';
        return v === null || v === undefined ? '' : String(v);
      }),
    );
}
