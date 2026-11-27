// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W0-kabuk 2026-11-27]
// Sayfa çizilmeden önce kayıtlı temayı uygular (karanlık temada beyaz parlama olmasın).
// Varsayılan bilerek "acik"tır (docs/TEMA.md: sistem izlenmez). Klasik betiktir; modül değildir.
(function () {
  var tema = 'acik';
  try {
    if (localStorage.getItem('taa-web-tema') === 'koyu') tema = 'koyu';
  } catch (e) {
    /* depolama kapalıysa varsayılan */
  }
  document.documentElement.setAttribute('data-tema', tema);
})();
