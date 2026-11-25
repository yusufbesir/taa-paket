/* TAA Terzi Takip ortak betik (masaüstü ve telefon): tema, ton, vurgu rengi, yazı boyutu.
   Saklama anahtarları: taa:terzitakip:tema | ton | renk:<tema> | yazi | tanitim-goruldu. localStorage yoksa
   (gizli pencere, WebView kısıtı) her şey varsayılanla çalışır. Sayfanın <head> bölümünde çalışır; ilk boyamadan
   önce temayı uygular. */
(function () {
  var kok = document.documentElement, A = 'taa:terzitakip:';
  var TEMALAR = ['kocan', 'tebesir', 'aski', 'cozgu', 'civit', 'keten'];
  /* Tebeşir ve Çivit yalnız koyudur (ton kilitli). */
  var SABIT_KOYU = { tebesir: 1, civit: 1 };
  function oku(k) { try { return localStorage.getItem(A + k); } catch (e) { return null; } }
  function yaz(k, v) { try { if (v == null) localStorage.removeItem(A + k); else localStorage.setItem(A + k, v); } catch (e) {} }
  function isik(h) {
    var c = [1, 3, 5].map(function (i) { var v = parseInt(h.substr(i, 2), 16) / 255; return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4); });
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
  }
  function hex(r) {
    r = String(r || '').trim();
    if (/^#[0-9a-f]{6}$/i.test(r)) return r.toUpperCase();
    if (/^#[0-9a-f]{3}$/i.test(r)) return ('#' + r[1] + r[1] + r[2] + r[2] + r[3] + r[3]).toUpperCase();
    var m = r.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
    if (m) return '#' + [m[1], m[2], m[3]].map(function (x) { return ('0' + (+x).toString(16)).slice(-2); }).join('').toUpperCase();
    return null;
  }
  function oran(a, b) { a = hex(a); b = hex(b); if (!a || !b) return 21; var x = isik(a), y = isik(b); return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05); }
  function jeton(ad) { return getComputedStyle(kok).getPropertyValue(ad).trim(); }
  function virgul(n) { return n.toFixed(1).replace('.', ','); }

  /* Vurgu rengi tema başına saklanır. Koçan'da zemine karşı 7, diğerlerinde 4,5; panel yüzeylerine karşı 4,5 gerekir. */
  function vurguDene(renk) {
    var esik = kok.dataset.tema === 'kocan' ? 7 : 4.5;
    var o = oran(renk, jeton('--tk-bg'));
    var o2 = Math.min(oran(renk, jeton('--tk-panel')), oran(renk, jeton('--tk-panel2')));
    return { tamam: o >= esik && o2 >= 4.5, zemin: o, panel: o2, esik: esik };
  }
  function vurguUygula(renk, sakla) {
    kok.style.removeProperty('--tk-ac'); kok.style.removeProperty('--tk-ac-fg');
    var tema = kok.dataset.tema;
    if (!renk) { if (sakla) yaz('renk:' + tema, null); return { tamam: true, sifir: true }; }
    var d = vurguDene(renk);
    if (!d.tamam) return d;
    kok.style.setProperty('--tk-ac', renk);
    kok.style.setProperty('--tk-ac-fg', oran(renk, '#000000') >= oran(renk, '#FFFFFF') ? '#000000' : '#FFFFFF');
    if (sakla) yaz('renk:' + tema, renk);
    return d;
  }
  function temaUygula(ad, ton, sakla) {
    if (TEMALAR.indexOf(ad) < 0) ad = 'kocan';
    kok.dataset.tema = ad;
    if (ton === 'koyu' && !SABIT_KOYU[ad]) kok.dataset.ton = 'koyu'; else delete kok.dataset.ton;
    if (sakla) { yaz('tema', ad); yaz('ton', kok.dataset.ton || null); }
    var kayitli = oku('renk:' + ad), sonuc = { tamam: true };
    kok.style.removeProperty('--tk-ac'); kok.style.removeProperty('--tk-ac-fg');
    if (kayitli) { sonuc = vurguUygula(kayitli, false); if (!sonuc.tamam) yaz('renk:' + ad, null); }
    /* Pencere başlık çubuğu (Edge/Chrome uygulama penceresi) üst bantla (--tk-panel2) aynı renkte olsun.
       <head> aşamasında da çalışır: renk jetonu :root'tan okunur, body'ye gerek yoktur. */
    var meta = document.querySelector('meta[name="theme-color"]');
    var bant = hex(jeton('--tk-baslik')) || hex(jeton('--tk-panel2')) || hex(jeton('--tk-bg'));
    if (meta && bant) meta.setAttribute('content', bant);
    return { tema: ad, koyu: kok.dataset.ton === 'koyu' || !!SABIT_KOYU[ad], vurguDustu: kayitli && !sonuc.tamam };
  }
  function yaziUygula(v, sakla) {
    if (v === 'buyuk') kok.dataset.yazi = 'buyuk'; else delete kok.dataset.yazi;
    if (sakla) yaz('yazi', v === 'buyuk' ? 'buyuk' : null);
  }
  function varsayilan() {
    TEMALAR.forEach(function (t) { yaz('renk:' + t, null); });
    yaz('tema', null); yaz('ton', null); yaz('yazi', null);
    yaziUygula('', false);
    return temaUygula('kocan', null, false);
  }
  var hareketAz = false;
  try { hareketAz = window.matchMedia('(prefers-reduced-motion: reduce)').matches; } catch (e) {}

  window.TK = {
    TEMALAR: TEMALAR, SABIT_KOYU: SABIT_KOYU, oku: oku, yaz: yaz, oran: oran, virgul: virgul,
    temaUygula: temaUygula, vurguUygula: vurguUygula, yaziUygula: yaziUygula, varsayilan: varsayilan,
    hareketAz: hareketAz
  };
  temaUygula(oku('tema') || 'kocan', oku('ton'), false);
  yaziUygula(oku('yazi') || '', false);
  /* <meta name="theme-color"> ilk çalıştırmada henüz ayrıştırılmamış olabilir; sayfa hazır olunca bir kez daha uygula. */
  document.addEventListener('DOMContentLoaded', function () { temaUygula(kok.dataset.tema, kok.dataset.ton, false); });
})();
