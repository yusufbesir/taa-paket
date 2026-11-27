// Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27]
// Testler için sahte `baglam` ({giden, onbellek, yetki, tema}); çekirdek sözleşmesini taklit eder:
// giden.ekle({program,uc,govde}) -> {no, durum:'bekliyor'}; giden.liste(durum); onbellek.oku/yaz. Silme yok.

export function sahteBaglam({ izinler = [], onbellek = {}, kullanici, sarmali = false } = {}) {
  const kutu = [];
  const okunan = [];
  const yazilan = [];
  const sorulan = [];
  let sayac = 0;
  const izinKumesi = new Set(izinler);
  const baglam = {
    giden: {
      async ekle({ program, uc, govde }) {
        sayac += 1;
        const kayit = { no: `SAHTE-${sayac}-1700000000`, program, uc, govde: structuredClone(govde), durum: 'bekliyor' };
        kutu.push(kayit);
        return { no: kayit.no, durum: 'bekliyor' };
      },
      async gonder() {
        throw new Error('modül gonder çağırmamalı: gönderim kabuğun işidir');
      },
      async liste(durum) {
        return kutu.filter((k) => !durum || k.durum === durum).map((k) => structuredClone(k));
      },
    },
    onbellek: {
      async oku(anahtar) {
        okunan.push(anahtar);
        if (!(anahtar in onbellek)) return undefined;
        const v = structuredClone(onbellek[anahtar]);
        return sarmali ? { deger: v, surum: 1 } : v;
      },
      async yaz(anahtar, deger, surum) {
        yazilan.push({ anahtar, deger, surum });
      },
    },
    yetki(izin) {
      sorulan.push(izin);
      return izinKumesi.has(izin);
    },
    tema: { ad: 'sinama', kip: 'acik' },
  };
  if (kullanici) baglam.kullanici = kullanici;
  return { baglam, kutu, okunan, yazilan, sorulan, izinKumesi };
}
