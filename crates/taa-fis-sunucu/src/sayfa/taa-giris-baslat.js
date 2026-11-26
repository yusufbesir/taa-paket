// Sahip: Yusuf. yazan: [Claude#ortak-giris 2026-11-25]
// TAA Terzi Takip masaüstü açılışında TAA ortak girişi (KM1). Electron'daki preload köprüsünün yerine
// window.taaGiris yerel sunucunun /api/giris/* uçlarına gider (yalnız bu bilgisayar + X-TAA-Masaustu).
// Kural: bu bilgisayarda taa-kimlik kuruluysa giriş zorunlu; kurulu değilse program eskisi gibi açılır.
// Geçerli oturum varsa ekran gösterilmez. Şifre bu dosyada tutulmaz, hiçbir yere yazılmaz.
(function () {
  'use strict';
  var ARALIK_MS = 10 * 60 * 1000;   // süren oturum aralıkla yeniden denetlenir

  function istek(islem, govde) {
    return fetch('/api/giris/' + islem, {
      method: 'POST', cache: 'no-store',
      headers: { 'X-TAA-Masaustu': '1', 'Content-Type': 'application/json' },
      body: JSON.stringify(govde || {})
    }).then(function (r) { if (!r.ok) throw new Error('giris ' + r.status); return r.json(); });
  }
  var api = Object.freeze({
    giris: function (kullanici, parola) { return istek('giris', { kullanici: kullanici, parola: parola }); },
    basvur: function (kullanici, yeniParola) { return istek('basvur', { kullanici: kullanici, yeniParola: yeniParola }); },
    baslangic: function () { return istek('baslangic'); },
    durum: function () { return istek('durum'); },
    cikis: function () { return istek('cikis'); }
  });

  // Terzi Takip temasını ortak ekranın değişkenlerine bağla (tema değişince kendiliğinden uyar).
  var stil = document.createElement('style');
  stil.textContent = ':root{--taa-arka:var(--tk-bg);--taa-panel:var(--tk-panel);--taa-yazi:var(--tk-tx);--taa-soluk:var(--tk-mut);' +
    '--taa-vurgu:var(--tk-ac);--taa-vurgu-yazi:var(--tk-ac-fg);--taa-cizgi:var(--tk-line);--taa-hata:var(--tk-hata);--taa-r:var(--tk-r)}' +
    '.taa-oturum-uyari{position:fixed;left:50%;bottom:24px;transform:translateX(-50%);max-width:560px;z-index:2147483000;' +
    'background:var(--tk-panel);color:var(--tk-tx);border:1px solid var(--tk-uyari);border-radius:var(--tk-r);padding:12px 44px 12px 16px;' +
    'box-shadow:var(--tk-gol);font:inherit;line-height:1.45}' +
    // Dar pencerede kenar çubuğu 64 px: hesap satırı yalnız baş harf rozeti olur.
    '@container pencere (max-width:1120px){.gez-alt .tg-hesap{padding:6px 0;justify-content:center;gap:0;width:100%}' +
    '.gez-alt .tg-hesap .tg-kim,.gez-alt .tg-hesap .tg-ok{display:none}}' +
    '.taa-oturum-uyari button{position:absolute;right:8px;top:8px;min-width:28px;min-height:28px;padding:0;border:0;background:none;color:inherit;font-size:18px;cursor:pointer}';
  document.head.appendChild(stil);

  function uyariGoster(oturum) {
    var metin = window.TaaGirisEkrani.uyariMetni(oturum);
    if (!metin) return;
    var d = document.createElement('div');
    d.className = 'taa-oturum-uyari';
    d.setAttribute('role', 'alert');
    d.textContent = metin;
    var k = document.createElement('button');
    k.type = 'button'; k.setAttribute('aria-label', 'Kapat'); k.textContent = '×';
    k.addEventListener('click', function () { d.remove(); });
    d.appendChild(k);
    document.body.appendChild(d);
  }

  // Ekran betiği yoksa sayfa kapatılır; program yeniden kurulmalı.
  function kilitle() {
    if (document.getElementById('taa-giris-kilit')) return;
    Array.prototype.forEach.call(document.body.children, function (e) { e.inert = true; });
    var d = document.createElement('div');
    d.id = 'taa-giris-kilit';
    d.setAttribute('role', 'alert');
    d.style.cssText = 'position:fixed;inset:0;z-index:2147483647;display:grid;place-items:center;padding:24px;background:var(--tk-bg);color:var(--tk-tx);font:inherit;text-align:center';
    d.textContent = 'Giriş ekranı yüklenemedi. TAA Terzi Takip\'i yeniden kurun ya da yöneticinize haber verin.';
    document.body.appendChild(d);
  }

  var TEMA_ADLARI = [['kocan', 'Koçan'], ['tebesir', 'Tebeşir'], ['aski', 'Askı'], ['cozgu', 'Çözgü'], ['civit', 'Çivit'], ['keten', 'Keten']];
  var sonHesap = null;   // menüsü kurulu kullanıcı (10 dakikalık denetim açık menüyü bozmasın)

  // Ortak hesap menüsü (kenar çubuğunun altında): hesap bilgisi, Kullanıcı değiştir, Ekranı kilitle, Çıkış yap + Terzi Takip öğeleri.
  // Öğeler menü her açıldığında yeniden kurulur (tema işareti, yönetici durumu güncel kalır).
  function cikisGoster(oturum) {
    if (!(window.TaaGirisEkrani && window.TaaGirisEkrani.hesapMenusu)) return;
    var ad = oturum && oturum.kullanici ? String(oturum.kullanici) : '';
    if (sonHesap === ad && document.getElementById('taa-giris-hesap')) return;
    sonHesap = ad;
    var k = function () { return window.TerziKopru || {}; };
    var kurulan = window.TaaGirisEkrani.hesapMenusu({
      api: api, oturum: oturum, cikinca: girisAc,
      hedef: document.querySelector('.gez-alt'), programAdi: 'TAA Terzi Takip',
      get ogeler() {
        var tema = document.documentElement.dataset.tema || 'kocan';
        // Sıra (Yusuf 11-26): kullanıcı bölümünün hemen altında Yönetici girişi (ya da Yönetici kipinden çık), ayraç, Ayarlar...
        // Yönetici kipi: kapalıyken PIN akışı açılır, açıkken yönetici oturumu düşer (ortak oturum sürer).
        var yonetici = (k().yoneticiAcik && k().yoneticiAcik())
          ? { ad: 'Yönetici kipinden çık', tikla: function () { k().yoneticiKilitle(); } }
          : { ad: 'Yönetici girişi', tikla: function () { if (k().git) k().git('telefonlar'); } };
        return [
          yonetici,
          { ayrac: true },
          { ad: 'Ayarlar', tikla: function () { if (k().git) k().git('ayarlar'); } },
          { ad: 'Tema', alt: TEMA_ADLARI.map(function (t) { return { ad: t[1], secili: t[0] === tema, tikla: function () { if (k().tema) k().tema(t[0]); } }; }) },
          { ayrac: true },
          { ad: 'Klavye kısayolları', kisayol: 'F1', tikla: function () { if (k().kisayollar) k().kisayollar(); } },
          { ad: 'Yardım ve sürüm', tikla: function () { if (k().yardim) k().yardim(); } }
        ];
      }
    });
    // Menü kurulduysa kenar çubuğundaki yedek Ayarlar/Yönetici girişi satırları gizlenir (masaustu.html CSS).
    if (kurulan) { var pen = document.querySelector('.pencere'); if (pen) pen.dataset.hesap = 'var'; }
  }

  function girildi(oturum) { uyariGoster(oturum); cikisGoster(oturum); }

  // n: hesap menüsünden gelen { neden, kullanici, ileti }; 'kilit' ise kullanıcı adı dolu açılır.
  function girisAc(n) {
    sonHesap = null;
    var d = document.getElementById('taa-giris-hesap'); if (d) d.remove();
    if (document.querySelector('.taa-giris')) return;   // zaten açık
    if (!window.TaaGirisEkrani) { kilitle(); return; }
    var s = { programAdi: 'TAA Terzi Takip', api: api, girildi: girildi };
    if (n && typeof n === 'object') { s.kullanici = n.kullanici; s.kilitli = n.neden === 'kilit'; if (n.ileti) s.ileti = n.ileti; }
    window.TaaGirisEkrani.ac(document.body, s);
  }

  function denetle() {
    return api.baslangic().then(function (b) {
      if (!b || !b.arac) return;          // taa-kimlik kurulu değil: eski davranış
      if (b.oturum) { cikisGoster(b.oturum); return; }   // geçerli oturum var
      girisAc();
    }).catch(function () { girisAc(); }); // emin olunamıyorsa giriş istenir
  }

  function basla() {
    denetle();
    setInterval(denetle, ARALIK_MS);
    // Başka programda çıkış yapıldıysa ortak bilgisayarda pencere öne gelince hemen fark edilsin (en çok 5 sn'de bir).
    var son = Date.now();
    function odakDenetle() {
      if (document.visibilityState === 'hidden' || Date.now() - son < 5000) return;
      son = Date.now();
      denetle();
    }
    window.addEventListener('focus', odakDenetle);
    document.addEventListener('visibilitychange', odakDenetle);
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', basla);
  else basla();
})();
