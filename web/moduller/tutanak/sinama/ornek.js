// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W3-tutanak 2026-11-27]
// Tutaa testleri için örnek şablonlar ve sunucu listesi (uydurma; gerçek kişi / olay verisi değildir).
// Şablon biçimi Tutaa şablonlarıyla aynıdır (bkz. crates/taa-kasa/sablonlar/kasa-cift-dogrulama.json).

export const SABLONLAR = () => ({
  sablonlar: [
    {
      kimlik: 'olay-tutanagi',
      ad: 'Olay / Tespit Tutanağı',
      sinif: 'tutanak',
      surum: 3,
      baslik: 'OLAY / TESPİT TUTANAĞI',
      bolumler: [
        {
          baslik: '1. Olay',
          alanlar: [
            { id: 'tarih', etiket: 'Olay Tarihi', tur: 'tarih', zorunlu: true },
            { id: 'saat', etiket: 'Olay Saati', tur: 'saat' },
            { id: 'yer', etiket: 'Yer', tur: 'metin', zorunlu: true },
            { id: 'tur', etiket: 'Olay Türü', tur: 'secim', zorunlu: true, secenekler: ['Tespit', 'Hasar', 'Diğer'] },
            { id: 'onlemler', etiket: 'Alınan Önlemler', tur: 'coklu', secenekler: ['Yöneticiye bildirildi', 'Kamera kaydı ayrıldı'] },
            { id: 'aciklama', etiket: 'Açıklama', tur: 'uzunmetin', zorunlu: true },
            { id: 'zarar', etiket: 'Tahmini Zarar (TL)', tur: 'sayi' },
            { id: 'acil', etiket: 'Acil', tur: 'onay' },
            { id: 'bilinmeyen', etiket: 'Bilinmeyen türde alan', tur: 'uydurma-tur' },
          ],
        },
      ],
      imzalar: [{ rol: 'Düzenleyen', imtinaKaydi: false }, { rol: 'Tanık', imtinaKaydi: false }, { rol: 'İlgili Personel', imtinaKaydi: true }],
    },
    {
      kimlik: 'sayim-fark',
      ad: 'Sayım Fark Tutanağı',
      surum: 1,
      bolumler: [{ baslik: 'Sayım', alanlar: [{ id: 'reyon', etiket: 'Reyon', tur: 'metin', zorunlu: true }] }],
      tablo: {
        baslik: 'Farklar',
        enAzSatir: 2,
        sutunlar: [
          { id: 'urun', etiket: 'Ürün', tur: 'metin', zorunlu: true },
          { id: 'kayit', etiket: 'Kayıtlı', tur: 'sayi' },
          { id: 'sayilan', etiket: 'Sayılan', tur: 'sayi' },
        ],
      },
      imzalar: [{ rol: 'Sayan' }, { rol: 'Mağaza Sorumlusu' }],
    },
    // Kasa'ya taşındı: bu modülde görünmemeli.
    {
      kimlik: 'kasa-cift-dogrulama',
      ad: 'Kasa Çift Doğrulama Tutanağı',
      surum: 2,
      bolumler: [{ baslik: 'Şube', alanlar: [{ id: 'sube', etiket: 'Şube Adı', tur: 'metin', zorunlu: true }] }],
      imzalar: [{ rol: 'Kasa Sorumlusu' }, { rol: 'Mağaza Sorumlusu / Vekili' }],
    },
    { kimlik: 'bozuk sablon', ad: 'Kimliği bozuk' },
    { kimlik: 'alansiz', ad: 'Alanı yok', bolumler: [] },
  ],
});

/** Sunucudan gelmiş (önbellekteki) tutanak özetleri. */
export const LISTE = () => ({
  tutanaklar: [
    { id: 41, no: 'MRK-OLY-2026-000041', sablon: 'olay-tutanagi', durum: 'acik', tarih: '2026-11-20', baslik: 'Depo kapısı', sha256: 'a'.repeat(64), imzalar: [{ rol: 'Düzenleyen', imzalayan: 'Örnek Düzenleyen' }] },
    { id: 42, no: 'MRK-OLY-2026-000042', sablon: 'olay-tutanagi', durum: 'acik', tarih: '2026-11-21', imzalar: [{ rol: 'Düzenleyen', imzalayan: 'Örnek Bir' }, { rol: 'Tanık', imzalayan: 'Örnek İki' }, { rol: 'İlgili Personel', imzalayan: 'Örnek Üç' }] },
    { id: 43, no: 'MRK-OLY-2026-000043', sablon: 'olay-tutanagi', durum: 'iptal', tarih: '2026-11-22', imzalar: [] },
    { id: 44, no: 'MRK-KCD-2026-000007', sablon: 'kasa-cift-dogrulama', durum: 'acik', tarih: '2026-11-23', imzalar: [] },
    { id: 45, no: '', sablon: '', durum: 'ocr_bekliyor', tarih: '2026-11-24', grup: 'f-sunucudaki', imzalar: [] },
  ],
});

/** En küçük geçerli başlıklara sahip sahte fotoğraf baytları (içerik resim değildir; yalnız tür denetimi için). */
export function sahteFoto(tur = 'jpeg', boyut = 64) {
  const b = new Uint8Array(boyut).fill(7);
  if (tur === 'jpeg') b.set([0xff, 0xd8, 0xff, 0xe0]);
  else if (tur === 'png') b.set([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  else if (tur === 'webp') b.set([0x52, 0x49, 0x46, 0x46, 0, 0, 0, 0, 0x57, 0x45, 0x42, 0x50]);
  else if (tur === 'pdf') b.set([0x25, 0x50, 0x44, 0x46, 0x2d]);
  return b;
}

/** Tarayıcı `File` nesnesinin testte kullanılan kadarı. */
export function sahteDosya(ad, bayt) {
  return { name: ad, size: bayt.length, type: '', arrayBuffer: async () => bayt.buffer.slice(bayt.byteOffset, bayt.byteOffset + bayt.byteLength) };
}

/** İmza kutusuna çizilmiş gibi nokta dizisi: `n` noktalı tek çizgi. */
export function ornekCizim(n = 12) {
  const c = [];
  for (let i = 0; i < n; i += 1) c.push(20 + i * 10, 100 + (i % 2) * 15);
  return { en: 600, boy: 200, cizgiler: [c] };
}
