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
      window.TaaGirisEkrani.cikisDugmesi({ api: api, oturum: oturum, cikinca: girisAc });
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
