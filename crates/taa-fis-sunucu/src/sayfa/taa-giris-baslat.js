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
    // Çıkış satırı: kenar çubuğunun en altında (Ayarlar'ın altında), durum çubuğuyla çakışmaz.
    '.gez-alt .tg-cikis{position:static;width:100%;min-height:var(--tk-hedef);margin:var(--tk-g1) 0 0;padding:0 var(--tk-g3);justify-content:flex-start;' +
    'border:0;border-top:1px solid var(--tk-line);border-radius:0;background:none;box-shadow:none;opacity:1;color:var(--tk-mut);' +
    'font:inherit;font-weight:600;overflow:hidden;white-space:nowrap;text-overflow:ellipsis;display:flex;align-items:center}' +
    '.gez-alt .tg-cikis:hover,.gez-alt .tg-cikis:focus-visible{background:var(--tk-panel2);color:var(--tk-tx)}' +
    '.gez-alt .tg-cikis:focus-visible{outline:3px solid var(--tk-ac);outline-offset:-3px}' +
    '@container pencere (max-width:1120px){.gez-alt .tg-cikis{justify-content:center;padding:0;font-size:0}.gez-alt .tg-cikis::before{content:"\\21AA";font-size:var(--tk-yazi)}}' +
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

  // Ortak bilgisayarda giren kişi kalıcı olmasın: köşede "Çıkış" düğmesi (basınca giriş yeniden istenir).
  function cikisGoster(oturum) {
    if (window.TaaGirisEkrani && window.TaaGirisEkrani.cikisDugmesi) {
      var d = window.TaaGirisEkrani.cikisDugmesi({ api: api, oturum: oturum, cikinca: girisAc });
      var alt = document.querySelector('.gez-alt');
      if (d && alt && d.parentNode !== alt) alt.appendChild(d);   // gövdeye sabit baloncuk yerine kenar çubuğu satırı
    }
  }

  function girildi(oturum) { uyariGoster(oturum); cikisGoster(oturum); }

  function girisAc() {
    var d = document.getElementById('taa-giris-cikis'); if (d) d.remove();
    if (document.querySelector('.taa-giris')) return;   // zaten açık
    if (!window.TaaGirisEkrani) { kilitle(); return; }
    window.TaaGirisEkrani.ac(document.body, { programAdi: 'TAA Terzi Takip', api: api, girildi: girildi });
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
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', basla);
  else basla();
})();
