// Sahip: Yusuf. yazan: [Claude#KM2-hesap-giris 2026-11-25]
// taa-giris-ekrani.js v2 — kaynak: yusufbesir/taa-paket/araclar/js/taa-giris-ekrani.js; programlar kopyalar, elle düzenlemez.
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
.taa-giris{transition:opacity .25s ease;position:fixed;inset:0;display:grid;place-items:center;padding:16px;z-index:2147483647;
  background:var(--taa-arka,#0f1420);color:var(--taa-yazi,#e8ecf3);font:15px/1.5 system-ui,-apple-system,"Segoe UI",sans-serif}
.taa-giris *{box-sizing:border-box}
.taa-giris .tg-kart{width:min(420px,100%);background:var(--taa-panel,#171e2c);border:1px solid var(--taa-cizgi,#2a3346);
  border-radius:var(--taa-r,14px);padding:28px 26px;box-shadow:0 18px 50px rgba(0,0,0,.25)}
.taa-giris h1{margin:0 0 4px;font-size:22px;font-weight:600}
.taa-giris.tg-gizli{visibility:hidden;opacity:0}
@media (prefers-reduced-motion:reduce){.taa-giris{transition:none}}
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
@keyframes taaGirisGel{from{opacity:0;transform:translateY(6px)}to{opacity:1;transform:none}}
.tg-hesap{position:fixed;left:12px;bottom:12px;z-index:2147482000;display:flex;align-items:center;gap:10px;margin:0;
  padding:6px 10px 6px 6px;min-height:0;height:auto;max-width:260px;font:13px/1.25 system-ui,-apple-system,"Segoe UI",sans-serif;
  cursor:pointer;text-align:left;color:var(--taa-yazi,#e8ecf3);background:var(--taa-panel,#171e2c);
  border:1px solid var(--taa-cizgi,#2a3346);border-radius:calc(var(--taa-r,14px) - 4px);box-shadow:0 4px 14px rgba(0,0,0,.18);
  text-transform:none;letter-spacing:normal;transform:none}
.tg-hesap.tg-yerinde{position:static;width:100%;max-width:none;box-shadow:none;background:none;border-color:transparent}
.tg-hesap:hover,.tg-hesap[aria-expanded="true"]{background:var(--taa-panel,#171e2c);border-color:var(--taa-cizgi,#2a3346);color:var(--taa-yazi,#e8ecf3)}
.tg-hesap:focus-visible,.tg-menu [role=menuitem]:focus-visible,.tg-menu [role=menuitemradio]:focus-visible{outline:2px solid var(--taa-vurgu,#f3a83b);outline-offset:1px}
.tg-rozet{flex:none;display:grid;place-items:center;width:32px;height:32px;border-radius:50%;font-weight:700;font-size:13px;
  background:var(--taa-vurgu,#f3a83b);color:var(--taa-vurgu-yazi,#1a1205)}
.tg-kim{display:grid;min-width:0}
.tg-kim b,.tg-kim span{overflow:hidden;white-space:nowrap;text-overflow:ellipsis}
.tg-kim b{font-weight:600}
.tg-kim span{font-size:12px;color:var(--taa-soluk,#9aa5b8)}
.tg-ok{margin-left:auto;color:var(--taa-soluk,#9aa5b8);font-size:11px}
.tg-menu{position:fixed;z-index:2147482500;min-width:260px;max-width:min(320px,calc(100vw - 16px));max-height:calc(100vh - 16px);overflow:auto;
  margin:0;padding:6px;font:14px/1.35 system-ui,-apple-system,"Segoe UI",sans-serif;color:var(--taa-yazi,#e8ecf3);
  background:var(--taa-panel,#171e2c);border:1px solid var(--taa-cizgi,#2a3346);border-radius:calc(var(--taa-r,14px) - 2px);
  box-shadow:0 18px 50px rgba(0,0,0,.3)}
.tg-menu *{box-sizing:border-box}
.tg-menu .tg-ust{display:flex;gap:10px;align-items:center;padding:8px 8px 10px}
.tg-menu .tg-ust .tg-rozet{width:40px;height:40px;font-size:15px}
.tg-menu .tg-sure{display:block;font-size:12px;color:var(--taa-soluk,#9aa5b8)}
.tg-menu hr{border:0;border-top:1px solid var(--taa-cizgi,#2a3346);margin:4px 2px}
.tg-menu [role=menuitem],.tg-menu [role=menuitemradio]{display:flex;align-items:center;gap:10px;width:100%;margin:0;padding:8px 10px;min-height:0;height:auto;
  font:inherit;text-align:left;color:inherit;background:none;border:0;border-radius:calc(var(--taa-r,14px) - 8px);cursor:pointer;
  box-shadow:none;text-transform:none;letter-spacing:normal;transform:none}
.tg-menu [role=menuitem]:hover,.tg-menu [role=menuitem]:focus,.tg-menu [role=menuitemradio]:hover,.tg-menu [role=menuitemradio]:focus{background:var(--taa-arka,#0f1420);outline:none}
.tg-menu kbd{margin-left:auto;font:12px ui-monospace,Consolas,monospace;color:var(--taa-soluk,#9aa5b8)}
.tg-menu .tg-tehlike{color:var(--taa-hata,#f06a5f)}
.tg-menu .tg-alt-menu{padding-left:16px}
.tg-menu .tg-alt-menu[hidden]{display:none}
.tg-menu .tg-cekmece .tg-secili{margin-left:auto;color:var(--taa-soluk,#9aa5b8);font-size:13px}
.tg-menu .tg-cekmece .tg-ok2{display:inline-block;margin-left:8px;color:var(--taa-soluk,#9aa5b8);transition:transform .15s ease}
.tg-menu .tg-cekmece[aria-expanded="true"] .tg-ok2{transform:rotate(90deg)}
.tg-menu [aria-checked="true"]::after{content:"✓";margin-left:auto;color:var(--taa-vurgu,#f3a83b)}
.tg-menu .tg-surum{padding:6px 10px 4px;font-size:11px;color:var(--taa-soluk,#9aa5b8)}
@media print{.tg-hesap,.tg-menu{display:none}}`;

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
    let sonKullanici = s.kullanici ? String(s.kullanici) : '';
    let kilitKipi = !!(s.kilitli && sonKullanici);
    const kilitliKullanici = kilitKipi ? sonKullanici : '';

    const kapat = () => {
      kap.querySelectorAll('input[type=password]').forEach((i) => { i.value = ''; });
      if (gozcu) gozcu.disconnect();
      inertler.forEach((e) => { e.inert = false; });
      kap.remove();
    };
    const ileti = (metin, tur) => { const e = kap.querySelector('.tg-ileti'); if (e) { e.textContent = metin || ''; e.className = 'tg-ileti tg-' + (tur || 'hata'); } };
    const odak = (sec) => { const e = kap.querySelector(sec); if (e) e.focus(); };

    function girisGoster(ilk) {
      const altMetin = kilitKipi ? 'Ekran kilitli. Devam etmek için şifrenizi yazın.' : 'TAA hesabınızla giriş yapın';
      kap.innerHTML = `<form class="tg-kart" novalidate>
        <h1 id="taa-giris-baslik">${programAdi}</h1><p class="tg-alt">${esc(altMetin)}</p>
        <label>Kullanıcı adı<input name="kullanici" autocomplete="username" autocapitalize="off" spellcheck="false" maxlength="64" required value="${esc(sonKullanici)}"${kilitKipi ? ' readonly aria-readonly="true"' : ''}></label>
        <label>Şifre<input name="parola" type="password" autocomplete="current-password" maxlength="128" required></label>
        <p class="tg-ileti" role="status" aria-live="polite"></p>
        <button class="tg-ana" type="submit">Giriş yap</button>
        <button class="tg-bag" type="button" data-basvur>Şifremi unuttum ya da hesabım kilitli</button>${kilitKipi ? '<button class="tg-bag" type="button" data-baska>Başka kullanıcıyla gir</button>' : ''}</form>`;
      if (ilk) ileti(ilk.metin, ilk.tur);
      const f = kap.querySelector('form');
      f.addEventListener('submit', async (e) => {
        e.preventDefault();
        const kullanici = kilitKipi ? kilitliKullanici : f.kullanici.value.trim();
        const parola = f.parola.value;
        if (!kullanici || !parola) { ileti('Kullanıcı adını ve şifreyi yazın.'); return; }
        sonKullanici = kullanici;
        const d = f.querySelector('.tg-ana');
        d.disabled = true; ileti('Giriş yapılıyor…', 'bilgi');
        let r;
        try { r = await api.giris(kullanici, parola); } catch (_) { r = { tamam: false, ileti: 'Giriş yapılamadı. Yeniden deneyin.' }; }
        f.parola.value = '';
        d.disabled = false;
        if (r && r.tamam) {
          const acildi = kilitKipi && r.oturum && r.oturum.kullanici === kilitliKullanici;
          kapat();
          if (acildi) olay(belge, 'taa-giris:kilit-acildi', { kullanici: kilitliKullanici });
          if (typeof s.girildi === 'function') s.girildi(r.oturum);
          return;
        }
        ileti((r && r.ileti) || 'Giriş yapılamadı.');
        if (r && r.durum === 'kilitli') { const b = f.querySelector('[data-basvur]'); b.textContent = 'Yöneticiye başvur'; b.focus(); } else odak('input[name=parola]');
      });
      f.querySelector('[data-basvur]').addEventListener('click', () => basvurGoster());
      // Kilitliyken başka kullanıcı: önce gerçek çıkış (ortak oturum ve yayın belirteçleri kapanır), sonra boş giriş.
      const baska = f.querySelector('[data-baska]');
      if (baska) baska.addEventListener('click', async () => {
        baska.disabled = true;
        let ilk;
        try { const y = await api.cikis(); if (y && y.tamam === false) ilk = { metin: 'Oturum bu bilgisayarda tam kapatılamadı.', tur: 'hata' }; }
        catch (_) { ilk = { metin: 'Oturum bu bilgisayarda tam kapatılamadı.', tur: 'hata' }; }
        olay(belge, 'taa-giris:cikildi', { kullanici: kilitliKullanici });
        kilitKipi = false; sonKullanici = '';
        girisGoster(ilk);
      });
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

    // ileti: açılışta gösterilecek tek satır (ör. hesap menüsünden gelen 'oturum tam kapatılamadı' uyarısı).
    girisGoster(s.ileti ? { metin: String(s.ileti), tur: 'hata' } : undefined);
    const ilkOdak = () => odak(sonKullanici ? 'input[name=parola]' : 'input[name=kullanici]');
    // gizli: true → ekran kurulur, arka sayfa kilitlenir ama görünmez; programın açılış animasyonu bitince goster() çağrılır.
    if (s.gizli) kap.classList.add('tg-gizli'); else ilkOdak();
    const goster = () => { if (kap.isConnected && kap.classList.contains('tg-gizli')) { kap.classList.remove('tg-gizli'); ilkOdak(); } };
    return { kapat, goster };
  }

  // Girişte hesap uyarısı varsa (başka bilgisayardan deneme, başvuruyla şifre değişikliği) program bunu gösterir.
  const uyariMetni = (oturum) => (oturum && oturum.uyari_sayisi > 0
    ? `Hesabınızla ilgili ${oturum.uyari_sayisi} yeni uyarı var: başka bir bilgisayardan giriş denemesi ya da şifre değişikliği. Siz değilseniz yöneticinize haber verin.`
    : '');

  const ROL_ADI = { yonetici: 'Yönetici', kasa: 'Kasa', personel: 'Personel', usta: 'Usta' };
  const basHarf = (ad) => (String(ad || '?').replace(/[^\p{L}\p{N}]/gu, ' ').trim().split(/\s+/).slice(0, 2)
    .map((p) => p[0]).join('') || '?').toLocaleUpperCase('tr-TR');
  const rolMetni = (o) => {
    const r = (o && Array.isArray(o.roller) ? o.roller : []).map((x) => ROL_ADI[x] || x);
    const sube = o && o.sube_id != null ? 'Şube ' + o.sube_id : '';
    return [r.join(', '), sube].filter(Boolean).join(' · ');
  };
  const sureMetni = (ms) => {
    const dk = Math.max(0, Math.floor(ms / 60000));
    return dk < 1 ? 'az önce' : dk < 60 ? dk + ' dk' : Math.floor(dk / 60) + ' sa ' + (dk % 60) + ' dk';
  };
  let girisZamani = null; // { kullanici, zaman }: bu pencerede ilk görülen giriş (oturum süresi için)
  // Programların dinleyebileceği olaylar (belge üzerinde, CustomEvent): taa-giris:kilitlendi, taa-giris:kilit-acildi,
  // taa-giris:cikildi. detail: { kullanici }.
  function olay(belge, ad, detay) {
    try { belge.dispatchEvent(new (belge.defaultView || kok).CustomEvent(ad, { detail: detay })); } catch (_) { /* eski ortam */ }
  }

  let kisayolKuruldu = false;
  let etkinMenu = null;   // son kurulan hesap menüsünün seçenekleri (Ctrl+L için)

  // Hesap menüsü (Yusuf 11-26): sol altta baş harf rozeti + ad + rol/şube; tıklayınca yukarı açılan menü.
  // Ortak öğeler: oturum süresi, Kullanıcı değiştir, Ekranı kilitle (Ctrl+L), onaylı Çıkış yap, sürüm.
  // Program kendi öğelerini `ogeler` ile ekler (Ayarlar, Tema alt menüsü, Kısayollar, Yönetici kipinden çık…).
  // Kullanıcı değiştir ve Çıkış yap ortak oturumu kapatır (taa-kimlik cikis; sunucudaki oturum da).
  // Ekranı kilitle (Yusuf 11-26 "Kilit yayını durdurmasın") oturumu KAPATMAZ: api.kilitle varsa ana süreç kilit işaretini
  //   yazar (program yeniden açılınca da kilitli açılır), yoksa yalnız arayüz kilitlenir. 'taa-giris:kilitlendi' olayı ve
  //   s.kilitlenince({ kullanici }) çağrılır. Kilit ekranında kullanıcı adı değişmez; "Başka kullanıcıyla gir" gerçek çıkış yapar.
  // Hepsinde cikinca({ neden, kullanici, ileti }) çağrılır:
  //   neden 'degistir' → boş giriş ekranı; 'kilit' → kullanıcı adı dolu giriş (ac(..., { kullanici, kilitli: true }));
  //   'cikis' → boş giriş ekranı. Program giriş ekranını kendisi açar.
//   ileti doluysa (çıkış tam olmadıysa) ac(..., { ileti }) ile girişte gösterilir.
  // secenek: { api, oturum, cikinca, kilitlenince?, hedef?, programAdi?, surum?, girisZamani?, ogeler? }
  //   hedef: düğmenin konacağı öğe (ör. kenar çubuğunun altı); yoksa ekranın sol altına sabitlenir.
  //   ogeler: [{ ad, kisayol?, tehlike?, tikla() } | { ayrac: true } | { ad, alt: [{ ad, secili?, tikla() }] }]
  //     ya da her açılışta çağrılan bir işlev (ör. yönetici kipine göre değişen öğeler için).
  // Yeniden çağrılırsa var olan düğme güncellenir.
  function hesapMenusu(secenek) {
    const s = secenek || {};
    const api = s.api || (kok && kok.taaGiris);
    const belge = (kok && kok.document) || document;
    if (!api || typeof api.cikis !== 'function' || !belge.body) return null;
    stilEkle(belge);
    menuKapat(belge);
    const oturum = s.oturum || {};
    const ad = oturum.kullanici ? String(oturum.kullanici) : '';
    if (s.girisZamani) girisZamani = { kullanici: ad, zaman: Number(s.girisZamani) };
    else if (!girisZamani || girisZamani.kullanici !== ad) girisZamani = { kullanici: ad, zaman: Date.now() };

    let d = belge.getElementById('taa-giris-hesap');
    const eski = belge.getElementById('taa-giris-cikis'); if (eski) eski.remove();
    if (!d) {
      d = belge.createElement('button');
      d.id = 'taa-giris-hesap';
      d.type = 'button';
      d.setAttribute('aria-haspopup', 'menu');
    }
    const kap = s.hedef && s.hedef.nodeType === 1 ? s.hedef : belge.body;
    if (d.parentNode !== kap) kap.appendChild(d);
    d.className = 'tg-hesap' + (kap === belge.body ? '' : ' tg-yerinde');
    d.setAttribute('aria-expanded', 'false');
    d.disabled = false;
    d.innerHTML = `<span class="tg-rozet" aria-hidden="true">${esc(basHarf(ad))}</span>
      <span class="tg-kim"><b>${esc(ad || 'Hesap')}</b><span>${esc(rolMetni(oturum))}</span></span><span class="tg-ok" aria-hidden="true">▴</span>`;
    d.setAttribute('aria-label', (ad ? ad + ' hesabı' : 'Hesap') + ' menüsü');
    d.onclick = () => (belge.getElementById('taa-giris-menu') ? menuKapat(belge, true) : menuAc());

    const ayar = { s, api, belge, d, ad, oturum };
    etkinMenu = ayar;
    if (!kisayolKuruldu) {
      kisayolKuruldu = true;
      belge.addEventListener('keydown', (e) => {
        if ((e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey && (e.key === 'l' || e.key === 'L') && etkinMenu
          && etkinMenu.d.isConnected && !etkinMenu.cikiyor && !belge.querySelector('.taa-giris')) {
          e.preventDefault();
          cik(etkinMenu, 'kilit');
        }
      });
    }

    function menuAc() {
      const m = belge.createElement('div');
      m.id = 'taa-giris-menu';
      m.className = 'tg-menu';
      m.setAttribute('role', 'menu');
      m.setAttribute('aria-label', 'Hesap');
      const sure = girisZamani ? sureMetni(Date.now() - girisZamani.zaman) : '';
      let html = `<div class="tg-ust" role="presentation"><span class="tg-rozet" aria-hidden="true">${esc(basHarf(ad))}</span>
        <span class="tg-kim"><b>${esc(ad || 'Hesap')}</b><span>${esc(rolMetni(oturum))}</span>
        <span class="tg-sure">${sure ? 'Oturum açık · ' + esc(sure) : ''}</span></span></div><hr>`;
      const islem = [];
      const oge = (o, ek) => { islem.push(o.tikla); return `<button type="button" role="menuitem" tabindex="-1" data-i="${islem.length - 1}"${ek || ''}${o.tehlike ? ' class="tg-tehlike"' : ''}>${esc(o.ad)}${o.kisayol ? '<kbd>' + esc(o.kisayol) + '</kbd>' : ''}</button>`; };
      let ogeListe = s.ogeler;
      if (typeof ogeListe === 'function') { try { ogeListe = ogeListe(); } catch (_) { ogeListe = []; } }
      (Array.isArray(ogeListe) ? ogeListe : []).forEach((o) => {
        if (!o) return;
        if (o.ayrac) { html += '<hr>'; return; }
        if (Array.isArray(o.alt)) {
          // Çekmece (Yusuf 11-26): kapalıyken tek satır "Tema · Çivit ›"; Enter/tık/sağ ok açar, sol ok/Esc kapatır.
          const secili = o.alt.find((a) => a && a.secili);
          html += `<div class="tg-grup"><button type="button" role="menuitem" tabindex="-1" class="tg-cekmece" aria-haspopup="true" aria-expanded="false">${esc(o.ad)}<span class="tg-secili">${secili ? esc(secili.ad) : ''}</span><span class="tg-ok2" aria-hidden="true">›</span></button><div class="tg-alt-menu" role="group" aria-label="${esc(o.ad)}" hidden>`;
          o.alt.forEach((a) => { if (!a) return; islem.push(a.tikla); html += `<button type="button" role="menuitemradio" tabindex="-1" aria-checked="${a.secili ? 'true' : 'false'}" data-i="${islem.length - 1}">${esc(a.ad)}</button>`; });
          html += '</div></div>';
          return;
        }
        html += oge(o);
      });
      if (islem.length) html += '<hr>';
      html += `<button type="button" role="menuitem" tabindex="-1" data-ortak="degistir">Kullanıcı değiştir</button>
        <button type="button" role="menuitem" tabindex="-1" data-ortak="kilit" title="Yalnız bu programın ekranı kilitlenir; diğer TAA programları açık kalır. Oturum sürer, süren işler (yayın gibi) durmaz. Açmak için şifre gerekir.">Ekranı kilitle<kbd>Ctrl+L</kbd></button>
        <hr><button type="button" role="menuitem" tabindex="-1" data-ortak="cikis" class="tg-tehlike">Çıkış yap</button>`;
      const alt = [s.programAdi, s.surum ? 'sürüm ' + s.surum : ''].filter(Boolean).join(' · ');
      if (alt) html += `<div class="tg-surum">${esc(alt)}</div>`;
      m.innerHTML = html;
      belge.body.appendChild(m);
      // Düğmenin üstüne aç; yer yoksa altına. Sola yasla, ekrandan taşma. Çekmece açılıp kapanınca yeniden yerleşir.
      const yerlestir = () => {
        const r = d.getBoundingClientRect();
        const g = belge.documentElement.clientWidth, y = belge.documentElement.clientHeight;
        const mr = m.getBoundingClientRect();
        m.style.left = Math.max(8, Math.min(r.left, g - mr.width - 8)) + 'px';
        m.style.top = Math.max(8, r.top - mr.height - 6 >= 8 ? r.top - mr.height - 6 : Math.min(r.bottom + 6, y - mr.height - 8)) + 'px';
      };
      yerlestir();
      const cekmece = (c, ac) => {
        const l = c.nextElementSibling;
        c.setAttribute('aria-expanded', ac ? 'true' : 'false');
        l.hidden = !ac;
        yerlestir();
        if (ac) { const sec = l.querySelector('[aria-checked="true"]') || l.querySelector('[role=menuitemradio]'); if (sec) sec.focus(); } else c.focus();
      };
      d.setAttribute('aria-expanded', 'true');
      const ogeler = () => Array.from(m.querySelectorAll('[role=menuitem],[role=menuitemradio]')).filter((e) => !e.closest('[hidden]'));
      m.addEventListener('click', (e) => {
        const b = e.target.closest('button'); if (!b) return;
        if (b.dataset.ortak === 'cikis' && !b.dataset.onay) {
          b.dataset.onay = '1'; b.textContent = 'Emin misiniz? Çıkmak için yeniden basın'; b.focus(); return;
        }
        if (b.dataset.ortak) { cik(ayar, b.dataset.ortak); return; }
        if (b.classList.contains('tg-cekmece')) { cekmece(b, b.getAttribute('aria-expanded') !== 'true'); return; }
        const f = islem[Number(b.dataset.i)];
        if (b.getAttribute('role') === 'menuitemradio') {
          // Seçim: işaret ve satırdaki ad güncellenir, çekmece kapanır; menü açık kalır.
          const l = b.parentNode;
          l.querySelectorAll('[role=menuitemradio]').forEach((x) => x.setAttribute('aria-checked', x === b ? 'true' : 'false'));
          const c = l.previousElementSibling;
          c.querySelector('.tg-secili').textContent = b.textContent;
          cekmece(c, false);
          if (typeof f === 'function') f();
          return;
        }
        menuKapat(belge, true);
        if (typeof f === 'function') f();
      });
      m.addEventListener('keydown', (e) => {
        const l = ogeler(); const i = l.indexOf(belge.activeElement);
        if (e.key === 'ArrowDown') { e.preventDefault(); l[(i + 1) % l.length].focus(); }
        else if (e.key === 'ArrowUp') { e.preventDefault(); l[(i - 1 + l.length) % l.length].focus(); }
        else if (e.key === 'Home') { e.preventDefault(); l[0].focus(); }
        else if (e.key === 'End') { e.preventDefault(); l[l.length - 1].focus(); }
        else if (e.key === 'ArrowRight' && belge.activeElement.classList.contains('tg-cekmece')) { e.preventDefault(); cekmece(belge.activeElement, true); }
        else if ((e.key === 'ArrowLeft' || e.key === 'Escape') && belge.activeElement.getAttribute('role') === 'menuitemradio') {
          e.preventDefault(); cekmece(belge.activeElement.parentNode.previousElementSibling, false);
        }
        else if (e.key === 'ArrowLeft' && belge.activeElement.classList.contains('tg-cekmece')) { e.preventDefault(); cekmece(belge.activeElement, false); }
        else if (e.key === 'Escape') { e.preventDefault(); menuKapat(belge, true); }
        else if (e.key === 'Tab') menuKapat(belge, false);
      });
      setTimeout(() => { if (m.isConnected) belge.addEventListener('pointerdown', disTik, true); }, 0);
      const ilk = ogeler()[0]; if (ilk) ilk.focus();
    }
    function disTik(e) {
      const m = belge.getElementById('taa-giris-menu');
      if (m && !m.contains(e.target) && !d.contains(e.target)) menuKapat(belge, false);
    }
    ayar.disTik = disTik;
    return d;
  }

  function menuKapat(belge, odakGeri) {
    const m = belge.getElementById('taa-giris-menu');
    if (m) m.remove();
    const d = belge.getElementById('taa-giris-hesap');
    if (d) { d.setAttribute('aria-expanded', 'false'); if (odakGeri) d.focus(); }
    if (etkinMenu && etkinMenu.disTik) belge.removeEventListener('pointerdown', etkinMenu.disTik, true);
  }

  async function cik(ayar, neden) {
    const { s, api, belge, d, ad } = ayar;
    if (ayar.cikiyor) return; // yeniden giriş yok: ikinci Ctrl+L ya da çift tık çıkışı iki kez çalıştırmaz
    ayar.cikiyor = true;
    menuKapat(belge, false);
    d.disabled = true;
    let ileti = '';
    if (neden === 'kilit') {
      // Oturum kapanmaz. Kilit işareti yazılamazsa yalnız arayüz kilitlenir (program yeniden açılınca kilit kalmaz).
      if (typeof api.kilitle === 'function') {
        try { const y = await api.kilitle(); if (!y || y.tamam !== true) ileti = 'Ekran kilitlendi; ancak program yeniden açılırsa kilit kalmayabilir.'; }
        catch (_) { ileti = 'Ekran kilitlendi; ancak program yeniden açılırsa kilit kalmayabilir.'; }
      }
    } else {
      try {
        const y = await api.cikis();
        if (y && y.tamam === false) ileti = 'Oturum bu bilgisayarda tam kapatılamadı; program yeniden açılınca kendiliğinden girebilir.';
      } catch (_) { ileti = 'Oturum bu bilgisayarda tam kapatılamadı; program yeniden açılınca kendiliğinden girebilir.'; }
      girisZamani = null;
    }
    d.remove();
    if (etkinMenu === ayar) etkinMenu = null;
    if (neden === 'kilit') {
      olay(belge, 'taa-giris:kilitlendi', { kullanici: ad });
      if (typeof s.kilitlenince === 'function') { try { s.kilitlenince({ kullanici: ad }); } catch (_) { /* program hatası kilidi bozmasın */ } }
    } else olay(belge, 'taa-giris:cikildi', { kullanici: ad });
    if (typeof s.cikinca === 'function') s.cikinca({ neden, kullanici: neden === 'kilit' ? ad : '', ileti });
  }

  // Eski ad (T3): aynı menüyü kurar.
  const cikisDugmesi = hesapMenusu;

  kok.TaaGirisEkrani = { ac, uyariMetni, hesapMenusu, cikisDugmesi };
})(typeof window !== 'undefined' ? window : globalThis);
