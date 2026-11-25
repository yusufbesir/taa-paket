// Sahip: Yusuf. yazan: [Claude#KM2-hesap-giris 2026-11-25]
// taa-giris-ekrani.js v1 — kaynak: yusufbesir/taa-paket/araclar/js/taa-giris-ekrani.js; programlar kopyalar, elle düzenlemez.
// TAA ortak giriş ekranı (renderer, tarayıcı tarafı; Node yok). window.taaGiris (taa-giris.js preloadKur) üzerinden çalışır.
// Sınıf adları tg- önekli: programın kendi .ana/.kart gibi kurallarıyla çakışmaz.
// Görünüm programın temasına uyar: renkler CSS değişkenlerinden okunur (--taa-arka, --taa-panel, --taa-yazi, --taa-soluk,
// --taa-vurgu, --taa-vurgu-yazi, --taa-cizgi, --taa-hata, --taa-r), yoksa sade varsayılanlar kullanılır.
//
// Kullanım:
//   index.html içine: script etiketiyle taa/taa-giris-ekrani.js yüklenir (satır içine gömülebilir; dosyada kapanan script etiketi yok).
//   TaaGirisEkrani.ac(document.body, { programAdi: 'Tutaa', girildi: (oturum) => { ... } });
//   Hedef ya document.body ya da boş, yalnız girişe ayrılmış bir kapsayıcı olmalı (hedefin içindeki başka öğeler inert yapılmaz).
// Akış: giriş → (kilit / şifremi unuttum) "Yöneticiye başvur" → iki haneli doğrulama sayısı ekranda büyük gösterilir.
// Şifre alanları ekran kapanınca boşaltılır; hiçbir şey localStorage'a yazılmaz.
(function (kok) {
  'use strict';
  const STIL_ID = 'taa-giris-stil';
  const STIL = `
.taa-giris{position:fixed;inset:0;display:grid;place-items:center;padding:16px;z-index:2147483647;
  background:var(--taa-arka,#0f1420);color:var(--taa-yazi,#e8ecf3);font:15px/1.5 system-ui,-apple-system,"Segoe UI",sans-serif}
.taa-giris *{box-sizing:border-box}
.taa-giris .tg-kart{width:min(420px,100%);background:var(--taa-panel,#171e2c);border:1px solid var(--taa-cizgi,#2a3346);
  border-radius:var(--taa-r,14px);padding:28px 26px;box-shadow:0 18px 50px rgba(0,0,0,.25)}
.taa-giris h1{margin:0 0 4px;font-size:22px;font-weight:600}
.taa-giris .tg-alt{margin:0 0 20px;color:var(--taa-soluk,#9aa5b8);font-size:13px}
.taa-giris label{display:block;margin:0 0 14px;font-size:13px;color:var(--taa-soluk,#9aa5b8)}
.taa-giris input{display:block;width:100%;margin-top:6px;padding:11px 12px;font:inherit;color:inherit;
  background:var(--taa-arka,#0f1420);border:1px solid var(--taa-cizgi,#2a3346);border-radius:calc(var(--taa-r,14px) - 6px)}
.taa-giris input:focus{outline:2px solid var(--taa-vurgu,#f3a83b);outline-offset:1px;border-color:transparent}
.taa-giris button{font:inherit;cursor:pointer;border-radius:calc(var(--taa-r,14px) - 6px);padding:11px 14px;
  min-height:0;height:auto;box-shadow:none;text-transform:none;letter-spacing:normal;transform:none}
.taa-giris .tg-ana{display:block;width:100%;border:0;font-weight:600;background:var(--taa-vurgu,#f3a83b);color:var(--taa-vurgu-yazi,#1a1205)}
.taa-giris .tg-ana:hover{background:var(--taa-vurgu,#f3a83b);filter:brightness(1.08)}
.taa-giris .tg-bag:hover,.taa-giris .tg-secim button:hover{background:none;filter:none}
.taa-giris .tg-ana[disabled]{opacity:.6;cursor:progress}
.taa-giris .tg-bag{display:block;width:100%;margin-top:12px;border:0;background:none;color:var(--taa-vurgu,#f3a83b);text-decoration:underline;padding:6px}
.taa-giris .tg-ileti{min-height:20px;margin:0 0 12px;font-size:13px}
.taa-giris .tg-ileti.tg-hata{color:var(--taa-hata,#f06a5f)}
.taa-giris .tg-ileti.tg-bilgi{color:var(--taa-soluk,#9aa5b8)}
.taa-giris .tg-sayi{margin:8px auto 16px;font:600 64px/1 ui-monospace,Consolas,monospace;letter-spacing:.25em;text-align:center;
  padding:18px 0 18px .25em;border:2px solid var(--taa-vurgu,#f3a83b);border-radius:var(--taa-r,14px)}
.taa-giris .tg-not{font-size:13px;color:var(--taa-soluk,#9aa5b8);margin:0 0 16px}
.taa-giris .tg-secim{display:flex;gap:8px;margin:0 0 16px}
.taa-giris .tg-secim button{flex:1;background:none;color:inherit;border:1px solid var(--taa-cizgi,#2a3346)}
.taa-giris .tg-secim button[aria-pressed="true"]{border-color:var(--taa-vurgu,#f3a83b);color:var(--taa-vurgu,#f3a83b)}
@media (prefers-reduced-motion:no-preference){.taa-giris .tg-kart{animation:taaGirisGel .22s cubic-bezier(.2,0,0,1)}}
@keyframes taaGirisGel{from{opacity:0;transform:translateY(6px)}to{opacity:1;transform:none}}`;

  const esc = (s) => String(s == null ? '' : s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));

  function stilEkle(belge) {
    if (belge.getElementById(STIL_ID)) return;
    const s = belge.createElement('style');
    s.id = STIL_ID;
    s.textContent = STIL;
    belge.head.appendChild(s);
  }

  function ac(hedef, secenek) {
    const s = secenek || {};
    const api = s.api || (kok && kok.taaGiris);
    if (!api) throw new Error('TaaGirisEkrani: window.taaGiris yok (preload taa-giris.js preloadKur çağrılmadı)');
    const belge = hedef.ownerDocument || document;
    stilEkle(belge);
    const kap = belge.createElement('div');
    kap.className = 'taa-giris';
    kap.setAttribute('role', 'dialog');
    kap.setAttribute('aria-modal', 'true');
    kap.setAttribute('aria-labelledby', 'taa-giris-baslik');
    hedef.appendChild(kap);
    // Arkadaki sayfa kullanılamaz: gövdenin diğer çocukları (sonradan eklenenler dahil) ekran kapanana dek inert.
    // Ekranı içeren öğe (hedef gövde değilse) inert yapılmaz.
    const inertler = [];
    const inertYap = (e) => { if (e !== kap && e.nodeType === 1 && !e.contains(kap) && !e.inert && e.tagName !== 'SCRIPT' && e.tagName !== 'STYLE') { e.inert = true; inertler.push(e); } };
    const govde = belge.body;
    if (govde) Array.from(govde.children).forEach(inertYap);
    const gozcu = govde && typeof MutationObserver === 'function'
      ? new MutationObserver((l) => l.forEach((m) => m.addedNodes.forEach(inertYap))) : null;
    if (gozcu) gozcu.observe(govde, { childList: true });
    const programAdi = esc(s.programAdi || 'TAA');
    let sonKullanici = '';

    const kapat = () => {
      kap.querySelectorAll('input[type=password]').forEach((i) => { i.value = ''; });
      if (gozcu) gozcu.disconnect();
      inertler.forEach((e) => { e.inert = false; });
      kap.remove();
    };
    const ileti = (metin, tur) => { const e = kap.querySelector('.tg-ileti'); if (e) { e.textContent = metin || ''; e.className = 'tg-ileti tg-' + (tur || 'hata'); } };
    const odak = (sec) => { const e = kap.querySelector(sec); if (e) e.focus(); };

    function girisGoster(ilk) {
      kap.innerHTML = `<form class="tg-kart" novalidate>
        <h1 id="taa-giris-baslik">${programAdi}</h1><p class="tg-alt">TAA hesabınızla giriş yapın</p>
        <label>Kullanıcı adı<input name="kullanici" autocomplete="username" autocapitalize="off" spellcheck="false" maxlength="64" required value="${esc(sonKullanici)}"></label>
        <label>Şifre<input name="parola" type="password" autocomplete="current-password" maxlength="128" required></label>
        <p class="tg-ileti" role="status" aria-live="polite"></p>
        <button class="tg-ana" type="submit">Giriş yap</button>
        <button class="tg-bag" type="button" data-basvur>Şifremi unuttum ya da hesabım kilitli</button></form>`;
      if (ilk) ileti(ilk.metin, ilk.tur);
      const f = kap.querySelector('form');
      f.addEventListener('submit', async (e) => {
        e.preventDefault();
        const kullanici = f.kullanici.value.trim();
        const parola = f.parola.value;
        if (!kullanici || !parola) { ileti('Kullanıcı adını ve şifreyi yazın.'); return; }
        sonKullanici = kullanici;
        const d = f.querySelector('.tg-ana');
        d.disabled = true; ileti('Giriş yapılıyor…', 'bilgi');
        let r;
        try { r = await api.giris(kullanici, parola); } catch (_) { r = { tamam: false, ileti: 'Giriş yapılamadı. Yeniden deneyin.' }; }
        f.parola.value = '';
        d.disabled = false;
        if (r && r.tamam) { kapat(); if (typeof s.girildi === 'function') s.girildi(r.oturum); return; }
        ileti((r && r.ileti) || 'Giriş yapılamadı.');
        if (r && r.durum === 'kilitli') { const b = f.querySelector('[data-basvur]'); b.textContent = 'Yöneticiye başvur'; b.focus(); } else odak('input[name=parola]');
      });
      f.querySelector('[data-basvur]').addEventListener('click', () => basvurGoster());
      odak(sonKullanici ? 'input[name=parola]' : 'input[name=kullanici]');
    }

    function basvurGoster() {
      let sifreUnuttum = true;
      kap.innerHTML = `<form class="tg-kart" novalidate>
        <h1 id="taa-giris-baslik">Yöneticiye başvur</h1>
        <p class="tg-alt">Başvurunuz yöneticiye gider. Onaylanınca giriş yapabilirsiniz.</p>
        <div class="tg-secim" role="group" aria-label="Başvuru nedeni">
          <button type="button" data-tur="sifre" aria-pressed="true">Şifremi unuttum</button>
          <button type="button" data-tur="kilit" aria-pressed="false">Hesabım kilitli</button></div>
        <label>Kullanıcı adı<input name="kullanici" autocomplete="username" autocapitalize="off" spellcheck="false" maxlength="64" required value="${esc(sonKullanici)}"></label>
        <div data-yeni>
          <label>Yeni şifre<input name="yeni" type="password" autocomplete="new-password" maxlength="128"></label>
          <label>Yeni şifre (tekrar)<input name="yeni2" type="password" autocomplete="new-password" maxlength="128"></label>
          <p class="tg-not">En az 8 karakter; kullanıcı adınızı içermesin.</p></div>
        <p class="tg-ileti" role="status" aria-live="polite"></p>
        <button class="tg-ana" type="submit">Başvuruyu gönder</button>
        <button class="tg-bag" type="button" data-geri>Girişe dön</button></form>`;
      const f = kap.querySelector('form');
      kap.querySelectorAll('[data-tur]').forEach((b) => b.addEventListener('click', () => {
        sifreUnuttum = b.dataset.tur === 'sifre';
        kap.querySelectorAll('[data-tur]').forEach((x) => x.setAttribute('aria-pressed', String(x === b)));
        kap.querySelector('[data-yeni]').hidden = !sifreUnuttum;
      }));
      f.addEventListener('submit', async (e) => {
        e.preventDefault();
        const kullanici = f.kullanici.value.trim();
        const yeni = sifreUnuttum ? f.yeni.value : '';
        if (!kullanici) { ileti('Kullanıcı adını yazın.'); return; }
        if (sifreUnuttum && (yeni.length < 8 || yeni !== f.yeni2.value)) {
          ileti(yeni.length < 8 ? 'Yeni şifre en az 8 karakter olmalı.' : 'İki şifre aynı değil.');
          return;
        }
        sonKullanici = kullanici;
        const d = f.querySelector('.tg-ana');
        d.disabled = true; ileti('Gönderiliyor…', 'bilgi');
        let r;
        try { r = await api.basvur(kullanici, yeni); } catch (_) { r = { tamam: false, ileti: 'Başvuru gönderilemedi. Yeniden deneyin.' }; }
        f.yeni.value = ''; f.yeni2.value = '';
        d.disabled = false;
        if (r && r.tamam) sayiGoster(r.dogrulama_sayisi, sifreUnuttum);
        else ileti((r && r.ileti) || 'Başvuru gönderilemedi.');
      });
      f.querySelector('[data-geri]').addEventListener('click', () => girisGoster());
      odak(sonKullanici ? (sifreUnuttum ? 'input[name=yeni]' : '.ana') : 'input[name=kullanici]');
    }

    function sayiGoster(sayi, sifre) {
      kap.innerHTML = `<div class="tg-kart">
        <h1 id="taa-giris-baslik">Başvurunuz iletildi</h1>
        <p class="tg-alt">Yöneticinize bu sayıyı söyleyin. Yönetici onaylarken sizden bu sayıyı isteyecek.</p>
        <div class="tg-sayi" aria-label="Doğrulama sayısı ${esc(sayi)}">${esc(sayi)}</div>
        <p class="tg-not">Sayıyı yalnız yöneticinize söyleyin. ${sifre ? 'Onaylanınca yeni şifrenizle giriş yapın.' : 'Onaylanınca eski şifrenizle giriş yapın.'} Yeniden başvurursanız sayı değişir.</p>
        <button class="tg-ana" type="button" data-tamam>Girişe dön</button></div>`;
      kap.querySelector('[data-tamam]').addEventListener('click', () => girisGoster({ metin: 'Yönetici onaylayınca giriş yapabilirsiniz.', tur: 'bilgi' }));
      odak('[data-tamam]');
    }

    girisGoster();
    return { kapat };
  }

  // Girişte hesap uyarısı varsa (başka bilgisayardan deneme, başvuruyla şifre değişikliği) program bunu gösterir.
  const uyariMetni = (oturum) => (oturum && oturum.uyari_sayisi > 0
    ? `Hesabınızla ilgili ${oturum.uyari_sayisi} yeni uyarı var: başka bir bilgisayardan giriş denemesi ya da şifre değişikliği. Siz değilseniz yöneticinize haber verin.`
    : '');

  kok.TaaGirisEkrani = { ac, uyariMetni };
})(typeof window !== 'undefined' ? window : globalThis);
