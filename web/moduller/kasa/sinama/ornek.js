// Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27]
// Kasa testleri için örnek mağaza ayarı ve farksız örnek gün (uydurma tutarlar; gerçek veri değildir).

export const AYAR = {
  sube: 'MRK',
  cekmeceler: [
    {
      id: 'C1',
      ad: 'Merkez çekmece',
      noktalar: [
        { id: 'N1', ad: 'Kasa 1', poslar: [{ id: 'Z1', ad: 'Ziraat POS', odeme_turu: 'ziraat_pos' }, { id: 'I1', ad: 'İyzico POS', odeme_turu: 'iyzico_pos' }] },
        { id: 'N2', ad: 'Kasa 2', poslar: [{ id: 'I2', ad: 'İyzico POS', odeme_turu: 'iyzico_pos' }] },
      ],
    },
  ],
};

/** Farksız örnek gün: açılış 500 TL, masraf 100 TL. */
export function ornekGun(degisiklik = {}) {
  return {
    tarih: '2026-11-27',
    cekmece: 'C1',
    nakit_adetler: { 20000: 7, 10000: 1 }, // 1.500 TL
    noktalar: [
      { nokta: 'N1', z_toplam_kurus: 250_000, pos: { Z1: 80_000, I1: 40_000 } },
      { nokta: 'N2', z_toplam_kurus: 110_000, pos: { I2: 30_000 } },
    ],
    nebim_toplam_kurus: 360_000,
    // nakit 1.000 + ziraat 800 + iyzico 700 + havale 1.000 = 3.500; + masraf 100 = 3.600
    ciro: { nakit: 100_000, ziraat_pos: 80_000, iyzico_pos: 70_000, havale: 100_000 },
    doviz: [],
    devreden_kurus: 50_000,
    kontrol: [true, true, true, true, true, true, true],
    not: '',
    aciklamalar: {},
    ...degisiklik,
  };
}
