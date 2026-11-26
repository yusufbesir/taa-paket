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
    var meta = document.querySelector('meta[name="theme-color"]');
    if (meta && document.body) meta.setAttribute('content', jeton('--tk-panel2') || jeton('--tk-bg'));
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
})();

/* ---------- Fiş düzeltme (masaüstü ve telefon ortak): window.TKD ----------
   Renk ve adet seçicileri, fiş ayrıntısındaki "Düzeltme" bölümü (işlem ekle, kalem çıkar, kargo/mağaza geçişi,
   fiş bağlama, bağlı fişler, düzeltme geçmişi) ve teslimde bağlı fiş sorusu (iletişim kutusu).
   Sunucudan gelen her metin textContent ile yazılır; innerHTML ile birleştirme yoktur. Asıl yetki denetimi
   sunucudadır (403 yetki_yok); arayüz yalnız oturumdaki "duzelt" yetkisine göre bölümü gösterir. */
(function () {
  var sayac = 0;
  function kimlik(on) { sayac += 1; return (on || 'dz') + '-' + sayac; }
  function ekle(e, c) {
    if (c == null || c === false) return;
    if (Array.isArray(c)) { c.forEach(function (x) { ekle(e, x); }); return; }
    e.appendChild(typeof c === 'string' ? document.createTextNode(c) : c);
  }
  /* el('button', {sinif:'dugme', metin:'Kaydet', tikla:fn, 'aria-label':'…'}, ...çocuklar) */
  function el(ad, oz) {
    var e = document.createElement(ad);
    oz = oz || {};
    Object.keys(oz).forEach(function (k) {
      var v = oz[k];
      if (v == null || v === false) return;
      if (k === 'metin') e.textContent = String(v);
      else if (k === 'sinif') e.className = v;
      else if (k === 'tikla') e.addEventListener('click', v);
      else if (k === 'degisti') e.addEventListener('change', v);
      else if (k === 'girdi') e.addEventListener('input', v);
      else e.setAttribute(k, v === true ? '' : String(v));
    });
    for (var i = 2; i < arguments.length; i++) ekle(e, arguments[i]);
    return e;
  }
  function hataIleti(e) { return (e && e.veri && e.veri.ileti) || (e && e.message) || 'Beklenmeyen hata'; }
  function yetkiVar(oturum, y) { return !!(oturum && oturum.yetkiler && oturum.yetkiler.indexOf(y) >= 0); }
  function renkAd(renkler, k) {
    if (!k) return '';
    for (var i = 0; i < (renkler || []).length; i++) if (renkler[i].anahtar === k) return renkler[i].ad;
    return k;
  }
  /* Kalem metnine eklenecek renk ve adet: " · Lacivert · 2 adet" (adet 1 ise yazılmaz). */
  function ekMetin(k, renkler) {
    var p = [];
    if (k && k.renk) p.push(renkAd(renkler, k.renk));
    if (k && k.adet > 1) p.push(k.adet + ' adet');
    return p.length ? ' · ' + p.join(' · ') : '';
  }
  function renkSecici(renkler, secili, oz) {
    var s = el('select', oz || {}, el('option', { value: '', metin: 'Renk seçilmedi' }));
    (renkler || []).forEach(function (r) { s.appendChild(el('option', { value: r.anahtar, metin: r.ad })); });
    s.value = secili || '';
    return s;
  }
  function adetSecici(secili, oz) {
    var s = el('select', oz || {});
    for (var n = 1; n <= 20; n++) s.appendChild(el('option', { value: String(n), metin: n === 1 ? '1 adet' : n + ' adet' }));
    s.value = String(Math.min(20, Math.max(1, +secili || 1)));
    return s;
  }
  /* Sunucuya giden kaleme renk ve adet: boş renk ve 1 adet gönderilmez. */
  function kalemeEkle(kalem, renk, adet) {
    if (renk) kalem.renk = renk;
    adet = parseInt(adet, 10);
    if (adet > 1) kalem.adet = Math.min(20, adet);
    return kalem;
  }
  var DURUM_AD = { bekliyor: 'Bekliyor', terzihanede: 'Terzihanede', hazir: 'Hazır', teslim: 'Teslim', iptal: 'İptal' };
  var NE_AD = { 'kalem-ekle': 'İşlem eklendi', 'kalem-cikar': 'Kalem çıkarıldı', 'teslim-sekli': 'Teslim şekli değişti', bagla: 'Fiş bağlandı', 'bag-coz': 'Bağ çözüldü' };
  function alan(etiket, girdi) {
    if (!girdi.id) girdi.id = kimlik('dzg');
    return el('div', { sinif: 'alan' }, el('label', { 'for': girdi.id, metin: etiket }), girdi);
  }
  function tlfGecerli(t) {
    var r = String(t || '').replace(/\D/g, '');
    if (r.length === 12 && r.indexOf('90') === 0) r = r.slice(2);
    return (r.length === 10 && r.charAt(0) === '5') || (r.length === 11 && r.indexOf('05') === 0);
  }

  /* Fişin düzeltilmeye açık alanlarının imzası: değişmediyse yenilemede bölüm (yazılanlarla) korunur.
     Bağlı fişlerin durumu ve müşteri adı da katılır (bağlı fiş hazır olunca listede eski durum kalmasın). */
  function imza(f, o) {
    var bagliDurum = (f.bagli || []).map(function (no) {
      var b = o && o.fisBul ? o.fisBul(no) : null;
      return b ? [b.durum, b.musteri] : null;
    });
    return JSON.stringify([f.no, f.durum, f.kalemler, f.kargo, f.kargo_ad, f.kargo_adres, f.kargo_telefon, f.kargo_notu, f.bagli, bagliDurum, f.duzeltmeler]);
  }
  var SON = null;

  /* Düzeltme bölümünü kap'a yerleştirir. o = {fis, api(metot, yol, govde) -> Promise, katalog, renkler,
     fisler() -> Promise<[fis]>, fisBul(no), fisAc(no), kisiAd(kim), sonra(ileti)}. Teslim/iptal fişte bölüm yoktur. */
  function yerlestir(kap, o) {
    if (!kap) return;
    var f = o.fis;
    if (!f || f.durum === 'teslim' || f.durum === 'iptal') { kap.textContent = ''; return; }
    var im = imza(f, o), odak = document.activeElement;
    if (SON && SON.imza === im && SON.dugum) {
      var icinde = SON.dugum.contains(odak);
      kap.textContent = '';
      kap.appendChild(SON.dugum);
      SON.o = o;
      if (icinde && odak && odak.focus) { try { odak.focus({ preventScroll: true }); } catch (e) { odak.focus(); } }
      return;
    }
    SON = { imza: im, o: o, dugum: bolum(o) };
    kap.textContent = '';
    kap.appendChild(SON.dugum);
  }

  function bolum(o) {
    var f = o.fis, yol = '/api/fis/' + encodeURIComponent(f.no), bas = kimlik('dzb'), formId = kimlik('dzf');
    var mesaj = el('div', { sinif: 'mesaj hata dz-mesaj', role: 'alert', hidden: true });
    var form = el('div', { sinif: 'dz-form kart duz yigin', id: formId, hidden: true });
    var acik = '';
    function ilet(t, tur) { mesaj.hidden = !t; mesaj.className = 'mesaj dz-mesaj ' + (tur || 'hata'); mesaj.textContent = t || ''; }
    function kapat() {
      acik = ''; form.hidden = true; form.textContent = '';
      hepsi.forEach(function (d) { d.setAttribute('aria-expanded', 'false'); });
    }
    function gonder(dugme, yolEk, govde, tamamIleti) {
      ilet('');
      if (dugme) dugme.disabled = true;
      return o.api('POST', yol + yolEk, govde).then(function (r) {
        var u = r && r.uyarilar && r.uyarilar.length ? ' (' + r.uyarilar.join(' · ') + ')' : '';
        SON = null; kapat();
        if (o.sonra) o.sonra(tamamIleti + u);
      }, function (e) {
        if (dugme) dugme.disabled = false;
        ilet(e && e.kod === 403 ? hataIleti(e) : 'Kaydedilmedi: ' + hataIleti(e));
      });
    }
    function ac(ad, dugme, kur) {
      if (acik === ad) { kapat(); return; }
      kapat(); acik = ad; ilet('');
      if (dugme) dugme.setAttribute('aria-expanded', 'true');
      kur(form); form.hidden = false;
      var ilk = form.querySelector('input,select,textarea,button');
      if (ilk) ilk.focus();
    }
    function sira() { return el('div', { sinif: 'dz-sira' }, [].slice.call(arguments)); }
    function vazgec() { return el('button', { type: 'button', sinif: 'dugme', metin: 'Vazgeç', tikla: kapat }); }

    /* 1) İşlem ekle: ürün (katalog grubu ya da serbest), katalogdan işlem, seçenek, renk, adet, ölçü */
    function ekleKur(k) {
      var islemler = (o.katalog && o.katalog.islemler) || [], gruplar = [];
      islemler.forEach(function (i) { if (gruplar.indexOf(i.grup) < 0) gruplar.push(i.grup); });
      var urun = el('select', {}, gruplar.map(function (g) { return el('option', { value: g, metin: g }); }), el('option', { value: '', metin: 'Diğer (yazın)' }));
      var urunSerbest = el('input', { maxlength: 40, autocomplete: 'off', placeholder: 'Ör. Gömlek' });
      var islem = el('select', {}), islemSerbest = el('input', { maxlength: 60, autocomplete: 'off', placeholder: 'Ör. Ütü' });
      var varyant = el('select', {}), renk = renkSecici(o.renkler, ''), adet = adetSecici(1);
      var olcu = el('input', { 'class': 'mono', inputmode: 'decimal', maxlength: 5, placeholder: 'cm (isteğe bağlı)' });
      var aUrunS = alan('Ürün adı', urunSerbest), aIslem = alan('İşlem', islem), aIslemS = alan('Yapılacak işlem', islemSerbest), aVar = alan('Seçenek', varyant);
      function bulKod(kod) { for (var i = 0; i < islemler.length; i++) if (islemler[i].kod === kod) return islemler[i]; return null; }
      function varyantCiz() {
        var is = bulKod(islem.value), v = (is && is.varyantlar) || [];
        varyant.textContent = '';
        v.forEach(function (x) { varyant.appendChild(el('option', { metin: x })); });
        aVar.hidden = !v.length;
      }
      function islemCiz() {
        var g = urun.value, l = islemler.filter(function (i) { return i.grup === g; }), serbest = !g || !l.length;
        islem.textContent = '';
        l.forEach(function (x) { islem.appendChild(el('option', { value: x.kod, metin: x.ad })); });
        aUrunS.hidden = !!g; aIslem.hidden = serbest; aIslemS.hidden = !serbest;
        varyantCiz();
      }
      urun.addEventListener('change', islemCiz);
      islem.addEventListener('change', varyantCiz);
      var kaydet = el('button', { type: 'button', sinif: 'dugme sonraki', metin: 'İşlemi ekle' });
      kaydet.addEventListener('click', function () {
        var g = urun.value, is = g ? bulKod(islem.value) : null, serbest = aIslem.hidden;
        var u = g || urunSerbest.value.trim(), ad = serbest ? islemSerbest.value.trim() : (is ? is.ad : '');
        if (!u) { ilet('Ürünü seçin ya da yazın.'); urunSerbest.focus(); return; }
        if (!ad) { ilet('İşlemi seçin ya da yazın.'); (serbest ? islemSerbest : islem).focus(); return; }
        var kalem = { urun: u, islem: ad };
        if (!serbest && is) kalem.islem_kodu = is.kod;
        if (!aVar.hidden && varyant.value) kalem.varyant = varyant.value;
        var cm = String(olcu.value || '').trim();
        if (cm) {
          var mm = Math.round(parseFloat(cm.replace(',', '.')) * 10);
          if (!(mm > 0)) { ilet('Ölçüyü santimetre olarak yazın ya da boş bırakın.'); olcu.focus(); return; }
          kalem.olcu_mm = mm;
        }
        kalemeEkle(kalem, renk.value, adet.value);
        gonder(kaydet, '/kalem', { kalem: kalem }, 'İşlem eklendi: ' + u + ' · ' + ad);
      });
      k.appendChild(el('h4', { metin: 'İşlem ekle' }));
      k.appendChild(el('p', { sinif: 'mut kucuk', metin: 'Fişe sonradan istenen işlemi ekler (ör. ütü). Ücreti ayrıca konuşun.' }));
      k.appendChild(el('div', { sinif: 'dz-izgara' }, alan('Ürün', urun), aUrunS, aIslem, aIslemS, aVar, alan('Renk', renk), alan('Adet', adet), alan('Ölçü', olcu)));
      k.appendChild(sira(vazgec(), kaydet));
      islemCiz();
    }

    /* 2) Kalem çıkar: neden sorar (sıra 0'dan) */
    function cikarKur(n) {
      return function (k) {
        var x = f.kalemler[n], neden = el('input', { maxlength: 200, autocomplete: 'off' });
        var tamam = el('button', { type: 'button', sinif: 'dugme tehlike', metin: 'Kalemi çıkar' });
        tamam.addEventListener('click', function () {
          var t = neden.value.trim();
          if (!t) { ilet('Çıkarma nedenini yazın.'); neden.focus(); return; }
          // Görülen kalem de gider: arada fiş değiştiyse (sıra kaydıysa) sunucu 409 döner, yanlış kalem çıkmaz.
          gonder(tamam, '/kalem/' + n + '/cikar', { neden: t, urun: x.urun || '', islem: x.islem || '' }, 'Kalem çıkarıldı: ' + x.urun + ' · ' + x.islem);
        });
        k.appendChild(el('h4', { metin: 'Kalem çıkar' }));
        k.appendChild(el('p', { metin: (n + 1) + '. ' + x.urun + ' · ' + x.islem + (x.varyant ? ' (' + x.varyant + ')' : '') + ekMetin(x, o.renkler) }));
        k.appendChild(alan('Neden (zorunlu)', neden));
        k.appendChild(sira(vazgec(), tamam));
      };
    }

    /* 3) Teslim şekli: kargo bilgileri formu; mağazadan teslime dönüşte neden zorunlu */
    function kargoKur(k) {
      var ad = el('input', { maxlength: 60, autocomplete: 'off' }), adres = el('textarea', { rows: 2, maxlength: 200 });
      var tel = el('input', { inputmode: 'tel', maxlength: 20, autocomplete: 'off', placeholder: '05xx xxx xx xx' }), not = el('input', { maxlength: 120, autocomplete: 'off' });
      ad.value = f.kargo ? (f.kargo_ad || '') : (f.musteri || '');
      adres.value = f.kargo_adres || '';
      tel.value = f.kargo ? (f.kargo_telefon || '') : (f.telefon || '');
      not.value = f.kargo_notu || '';
      var neden = el('input', { maxlength: 200, autocomplete: 'off' });
      var kaydet = el('button', { type: 'button', sinif: 'dugme sonraki', metin: f.kargo ? 'Kargo bilgisini kaydet' : 'Kargo ile gönder' });
      kaydet.addEventListener('click', function () {
        if (ad.value.trim().split(/\s+/).length < 2) { ilet('Alıcının adını ve soyadını yazın.'); ad.focus(); return; }
        if (adres.value.trim().length < 15) { ilet('Açık adresi yazın (mahalle, sokak, no, ilçe/il).'); adres.focus(); return; }
        if (!tlfGecerli(tel.value)) { ilet('Alıcı telefonu geçersiz: 05xx xxx xx xx biçiminde yazın.'); tel.focus(); return; }
        gonder(kaydet, '/teslim-sekli', { kargo: true, kargo_ad: ad.value.trim(), kargo_adres: adres.value.trim(), kargo_telefon: tel.value.trim(),
          kargo_notu: not.value.trim(), neden: neden.value.trim() }, f.kargo ? 'Kargo bilgisi güncellendi' : 'Fiş kargo ile gönderilecek');
      });
      k.appendChild(el('h4', { metin: f.kargo ? 'Kargo bilgisi' : 'Kargo ile gönder' }));
      k.appendChild(el('div', { sinif: 'dz-izgara' }, alan('Alıcı adı soyadı', ad), alan('Alıcı telefonu', tel)));
      k.appendChild(alan('Açık adres', adres));
      k.appendChild(el('div', { sinif: 'dz-izgara' }, alan('Kargo notu', not), alan('Neden (isteğe bağlı)', neden)));
      k.appendChild(sira(vazgec(), kaydet));
    }
    function magazaKur(k) {
      var neden = el('input', { maxlength: 200, autocomplete: 'off' });
      var tamam = el('button', { type: 'button', sinif: 'dugme tehlike', metin: 'Mağazadan teslime çevir' });
      tamam.addEventListener('click', function () {
        var t = neden.value.trim();
        if (!t) { ilet('Kargonun neden iptal edildiğini yazın.'); neden.focus(); return; }
        gonder(tamam, '/teslim-sekli', { kargo: false, kargo_ad: '', kargo_adres: '', kargo_telefon: '', kargo_notu: '', neden: t }, 'Kargo iptal edildi; ürün mağazadan teslim edilecek');
      });
      k.appendChild(el('h4', { metin: 'Mağazadan teslim' }));
      k.appendChild(el('p', { sinif: 'mut kucuk', metin: 'Kargo bilgileri silinir; ürün mağazadan teslim edilir.' }));
      k.appendChild(alan('Kargo neden iptal edildi? (zorunlu)', neden));
      k.appendChild(sira(vazgec(), tamam));
    }

    /* 4) Fişi bağla: /api/fisler'den fiş no ya da müşteri adına göre arama */
    function baglaKur(k) {
      var ara = el('input', { type: 'search', autocomplete: 'off', placeholder: 'Fiş no ya da müşteri adı' });
      var sonuc = el('ul', { sinif: 'dz-sonuc', 'aria-live': 'polite' }), liste = null, secili = '';
      var tamam = el('button', { type: 'button', sinif: 'dugme sonraki', metin: 'Bağla', disabled: true });
      function ciz() {
        sonuc.textContent = '';
        if (!liste) { sonuc.appendChild(el('li', { sinif: 'mut', metin: 'Fişler yükleniyor…' })); return; }
        var q = ara.value.toLocaleLowerCase('tr').trim();
        var l = liste.filter(function (x) {
          if (x.no === f.no || x.durum === 'teslim' || x.durum === 'iptal' || (f.bagli || []).indexOf(x.no) >= 0) return false;
          return !q || (x.no + ' ' + (x.musteri || '')).toLocaleLowerCase('tr').indexOf(q) >= 0;
        }).slice(0, 8);
        if (!l.length) { sonuc.appendChild(el('li', { sinif: 'mut', metin: q ? 'Aramaya uyan açık fiş yok.' : 'Bağlanabilecek açık fiş yok.' })); return; }
        l.forEach(function (x) {
          var d = el('button', { type: 'button', sinif: 'dz-secim', 'aria-pressed': String(x.no === secili) },
            el('b', { sinif: 'mono', metin: x.no }), ' ', el('span', { metin: x.musteri || '' }), ' ', el('span', { sinif: 'mut kucuk', metin: DURUM_AD[x.durum] || x.durum }));
          d.addEventListener('click', function () { secili = x.no; tamam.disabled = false; tamam.textContent = x.no + ' ile bağla'; ciz(); });
          sonuc.appendChild(el('li', {}, d));
        });
      }
      ara.addEventListener('input', ciz);
      tamam.addEventListener('click', function () { if (secili) gonder(tamam, '/bagla', { diger: secili }, f.no + ' ile ' + secili + ' bağlandı'); });
      k.appendChild(el('h4', { metin: 'Fişi bağla' }));
      k.appendChild(el('p', { sinif: 'mut kucuk', metin: 'Birlikte teslim edilecek fişleri bağlar (ör. birlikte gelen müşteriler). Teslimde bağlı fişler sorulur.' }));
      k.appendChild(alan('Ara', ara));
      k.appendChild(sonuc);
      k.appendChild(sira(vazgec(), tamam));
      ciz();
      Promise.resolve(o.fisler ? o.fisler() : []).then(function (l) { liste = l || []; ciz(); }, function (e) { liste = []; ilet('Fişler okunamadı: ' + hataIleti(e)); ciz(); });
    }

    var dEkle = el('button', { type: 'button', sinif: 'dugme', metin: 'İşlem ekle', 'aria-expanded': 'false', 'aria-controls': formId });
    dEkle.addEventListener('click', function () { ac('ekle', dEkle, ekleKur); });
    var dKargo = el('button', { type: 'button', sinif: 'dugme', metin: f.kargo ? 'Mağazadan teslim' : 'Kargo ile gönder', 'aria-expanded': 'false', 'aria-controls': formId });
    dKargo.addEventListener('click', function () { ac('sekil', dKargo, f.kargo ? magazaKur : kargoKur); });
    var dKargoDuz = f.kargo ? el('button', { type: 'button', sinif: 'dugme', metin: 'Kargo bilgisini düzelt', 'aria-expanded': 'false', 'aria-controls': formId }) : null;
    if (dKargoDuz) dKargoDuz.addEventListener('click', function () { ac('kargo', dKargoDuz, kargoKur); });
    var dBagla = el('button', { type: 'button', sinif: 'dugme', metin: 'Fişi bağla', 'aria-expanded': 'false', 'aria-controls': formId });
    dBagla.addEventListener('click', function () { ac('bagla', dBagla, baglaKur); });
    var dCoz = (f.bagli || []).length ? el('button', { type: 'button', sinif: 'dugme tehlike', metin: 'Bağı çöz' }) : null;
    if (dCoz) dCoz.addEventListener('click', function () {
      if (!window.confirm(f.no + ' bağlı fiş grubundan çıkarılsın mı?')) return;
      gonder(dCoz, '/bag-coz', {}, f.no + ': bağ çözüldü');
    });
    var dugmeler = [dEkle, dKargo, dKargoDuz, dBagla].filter(Boolean), hepsi = dugmeler.slice();

    /* kalemler: her biri "Çıkar" (tek kalemde devre dışı) */
    var tek = (f.kalemler || []).length < 2;
    var kalemler = el('ul', { sinif: 'dz-liste' }, (f.kalemler || []).map(function (x, n) {
      var d = el('button', { type: 'button', sinif: 'dugme kucuk tehlike', metin: 'Çıkar', disabled: tek, 'aria-expanded': 'false', 'aria-controls': formId,
        'aria-label': (n + 1) + '. kalemi çıkar: ' + x.urun + ' ' + x.islem, title: tek ? 'Fişte tek kalem var; çıkarmak yerine fişi iptal edin' : null });
      d.addEventListener('click', function () { ac('cikar' + n, d, cikarKur(n)); });
      hepsi.push(d);
      return el('li', {}, el('span', { metin: (n + 1) + '. ' + x.urun + ' · ' + x.islem + (x.varyant ? ' (' + x.varyant + ')' : '') + ekMetin(x, o.renkler) }), d);
    }));

    /* bağlı fişler */
    var bagli = (f.bagli || []).length ? el('ul', { sinif: 'dz-liste' }, f.bagli.map(function (no) {
      var b = o.fisBul ? o.fisBul(no) : null, li = el('li', {}, el('span', {}, el('b', { sinif: 'mono', metin: no }), b ? ' · ' + (b.musteri || '') + ' · ' + (DURUM_AD[b.durum] || b.durum) : ''));
      if (o.fisAc) li.appendChild(el('button', { type: 'button', sinif: 'dugme kucuk', metin: 'Aç', 'aria-label': no + ' fişini aç', tikla: function () { o.fisAc(no); } }));
      return li;
    })) : el('p', { sinif: 'mut kucuk', metin: 'Bağlı fiş yok.' });

    /* düzeltme geçmişi (yeniden eskiye) */
    var gecmis = (f.duzeltmeler || []).length ? el('ol', { sinif: 'dz-gecmis', reversed: true }, f.duzeltmeler.slice().reverse().map(function (d) {
      var kim = o.kisiAd ? o.kisiAd(d.kim) : d.kim;
      return el('li', {}, el('b', { metin: NE_AD[d.ne] || d.ne }), d.ayrinti ? ': ' + d.ayrinti : '', el('br'), el('span', { sinif: 'mut kucuk', metin: (d.tarih || '') + (kim ? ' · ' + kim : '') }));
    })) : el('p', { sinif: 'mut kucuk', metin: 'Henüz düzeltme yok.' });

    return el('section', { sinif: 'dz yigin', 'aria-labelledby': bas },
      el('div', { sinif: 'dz-bas' }, el('h3', { id: bas, metin: 'Düzeltme' }), el('span', { sinif: 'mut kucuk', metin: 'Her değişiklik fişin geçmişine yazılır' })),
      mesaj,
      el('div', { sinif: 'dz-dugmeler', role: 'group', 'aria-label': 'Düzeltme işlemleri' }, dugmeler, dCoz),
      form,
      el('h4', { metin: 'Kalemler' }), kalemler,
      el('h4', { metin: 'Bağlı fişler' }), bagli,
      el('h4', { metin: 'Düzeltme geçmişi' }), gecmis);
  }

  /* Teslimde bağlı fiş sorusu (sunucu 409 {soru:"bagli_teslim", ileti, bagli:[{no,musteri,durum,hazir}]}).
     2026-11-26: teslim arayüzü artık teslim sepetini kullanır (bağlı fişler sepete kendiliğinden gelir); bu kutu yalnız
     eski POST /api/fis/<no>/teslim-al ucunu doğrudan çağıran istemciler için durur.
     Sonuç: seçilen hazır fiş no'ları (boş dizi = yalnız bu fiş) ya da vazgeçilirse null.
     s.ekDugmeler: [{metin, sinif, tikla(kapat)}] ek düğmeler (Ekip B2: fişsiz teslim / imza). */
  function bagliSor(v, s) {
    s = s || {};
    return new Promise(function (bitti) {
      var bas = kimlik('dzk'), acik = kimlik('dzk'), sonuc = null, onceki = document.activeElement;
      var kutular = (v.bagli || []).map(function (x) {
        var c = el('input', { type: 'checkbox', value: x.no, disabled: !x.hazir });
        c.checked = !!x.hazir;
        return { x: x, c: c };
      });
      var secilen = el('button', { type: 'button', sinif: 'dugme sonraki', metin: 'Seçilenlerle teslim et' });
      function guncelle() { secilen.disabled = !kutular.some(function (k) { return k.c.checked; }); }
      kutular.forEach(function (k) { k.c.addEventListener('change', guncelle); });
      var kutu = el('dialog', { sinif: 'dz-kutu', 'aria-labelledby': bas, 'aria-describedby': acik },
        el('h2', { id: bas, metin: s.baslik || 'Bağlı fişler' }),
        el('p', { id: acik, metin: v.ileti || 'Bu fişe bağlı başka fişler var. Onlar da şimdi teslim edilsin mi?' }),
        el('fieldset', { sinif: 'dz-kutu-liste' }, el('legend', { sinif: 'sr', metin: 'Birlikte teslim edilecek fişler' }), kutular.map(function (k) {
          return el('label', { sinif: 'onay' + (k.x.hazir ? '' : ' dz-pasif') }, k.c, el('span', {},
            el('b', { sinif: 'mono', metin: k.x.no }), ' · ' + (k.x.musteri || ''),
            k.x.hazir ? '' : el('span', { sinif: 'dz-neden', metin: ' · Hazır değil (' + (DURUM_AD[k.x.durum] || k.x.durum) + '); birlikte teslim edilemez' })));
        })));
      function kapat(r) { sonuc = r; if (kutu.open && kutu.close) kutu.close(); else son(); }
      var bitmis = false;
      function son() { if (bitmis) return; bitmis = true; if (kutu.parentNode) kutu.parentNode.removeChild(kutu); if (onceki && onceki.focus) try { onceki.focus(); } catch (e) {} bitti(sonuc); }
      secilen.addEventListener('click', function () { kapat(kutular.filter(function (k) { return k.c.checked; }).map(function (k) { return k.x.no; })); });
      var dugmeler = el('div', { sinif: 'dz-sira' },
        el('button', { type: 'button', sinif: 'dugme', metin: 'Vazgeç', tikla: function () { kapat(null); } }),
        (s.ekDugmeler || []).map(function (d) { return el('button', { type: 'button', sinif: d.sinif || 'dugme', metin: d.metin, tikla: function () { d.tikla(kapat); } }); }),
        el('button', { type: 'button', sinif: 'dugme', metin: 'Yalnız bu fiş', tikla: function () { kapat([]); } }),
        secilen);
      kutu.appendChild(dugmeler);
      kutu.addEventListener('close', son);
      kutu.addEventListener('cancel', function () { sonuc = null; });
      guncelle();
      document.body.appendChild(kutu);
      if (kutu.showModal) kutu.showModal(); else kutu.setAttribute('open', '');
      (kutular.filter(function (k) { return !k.c.disabled; })[0] || { c: secilen }).c.focus();
    });
  }

  /* ---------- Ekip B2: teslim alan, fişsiz teslim ve imza ----------
     Sunucu ucu (2026-11-26'dan beri arayüzde): POST /api/teslim/sepet {fisler, teslim_alan, iliski, telefon_son4?,
     imza_png_base64?, onay_belirteci?}. Eski uç POST /api/fis/<no>/teslim-al geri uyum için durur.
     Son 4 hane yalnız istek gövdesinde gider; arayüz onu hiçbir yerde saklamaz, göstermez. */
  /* Ad soyad karakterleri (sunucudaki ad_karakterleri_gecerli ile aynı): harf, boşluk, tire, kesme işareti, nokta. */
  var AD_DESENI = /^[\p{Alphabetic} .'’-]+$/u;
  var ILISKI = [['kendisi', 'Kendisi'], ['yakini', 'Yakını'], ['diger', 'Diğer']];
  function iliskiAd(k) { for (var i = 0; i < ILISKI.length; i++) if (ILISKI[i][0] === k) return ILISKI[i][1]; return k || ''; }
  var IMZA_SINIR = 300 * 1024; // sunucudaki sınır (çözülmüş PNG)

  /* İmza alanı: <canvas>, pointer olaylarıyla parmak/kalem/fare. Çizgiler CSS pikseliyle saklanır; boyut ya da
     devicePixelRatio değişince yeniden çizilir (tuval silinmez). Kâğıt gibi beyaz zemin, koyu mürekkep: imza
     her temada aynı görünür ve basılabilir. */
  function imzaAlani(s) {
    s = s || {};
    var cizgiler = [], simdiki = null, olcek = 1;
    // Not: imza yalnız işaretçiyle (dokunmatik ekran, kalem, fare) atılır; klavyeyle imza seçeneği yoktur.
    var tuval = el('canvas', { sinif: 'tk-imza-tuval', role: 'img', tabindex: '-1', 'aria-label': s.etiket || 'İmza alanı: imza dokunmatik ekranda parmakla ya da kalemle, veya fareyle atılır' });
    var ipucu = el('span', { sinif: 'tk-imza-ipucu', 'aria-hidden': 'true', metin: 'Buraya imzalayın' });
    var temizle = el('button', { type: 'button', sinif: 'dugme kucuk', metin: 'Temizle' });
    var kok = el('div', { sinif: 'tk-imza' }, el('div', { sinif: 'tk-imza-cerceve' }, tuval, ipucu), el('div', { sinif: 'tk-imza-alt' },
      el('span', { sinif: 'mut kucuk', metin: s.alt || 'İmza yalnız bu fişin teslim kaydına eklenir.' }), temizle));
    function boyutla() {
      var r = tuval.getBoundingClientRect(), d = window.devicePixelRatio || 1;
      if (!r.width || !r.height) return;
      var w = Math.round(r.width * d), h = Math.round(r.height * d);
      if (tuval.width !== w || tuval.height !== h) { tuval.width = w; tuval.height = h; }
      olcek = d; ciz();
    }
    function ciz() {
      var c = tuval.getContext('2d');
      c.setTransform(1, 0, 0, 1, 0, 0);
      c.fillStyle = '#FFFFFF'; c.fillRect(0, 0, tuval.width, tuval.height);
      c.setTransform(olcek, 0, 0, olcek, 0, 0);
      c.strokeStyle = '#111111'; c.fillStyle = '#111111'; c.lineCap = 'round'; c.lineJoin = 'round';
      cizgiler.forEach(function (n) {
        if (n.length === 1) { c.beginPath(); c.arc(n[0][0], n[0][1], n[0][2] / 2, 0, Math.PI * 2); c.fill(); return; }
        for (var i = 1; i < n.length; i++) {
          c.beginPath(); c.lineWidth = n[i][2]; c.moveTo(n[i - 1][0], n[i - 1][1]); c.lineTo(n[i][0], n[i][1]); c.stroke();
        }
      });
      ipucu.hidden = cizgiler.length > 0;
    }
    function nokta(e) {
      var r = tuval.getBoundingClientRect();
      // Kalem basıncı varsa çizgi kalınlığına yansır; fare/parmakta sabit kalınlık.
      var p = e.pointerType === 'pen' && e.pressure > 0 ? 1.2 + e.pressure * 2.4 : 2.4;
      return [e.clientX - r.left, e.clientY - r.top, p];
    }
    tuval.addEventListener('pointerdown', function (e) {
      if (e.button > 0) return;
      e.preventDefault();
      try { tuval.setPointerCapture(e.pointerId); } catch (x) {}
      boyutla();
      simdiki = [nokta(e)]; cizgiler.push(simdiki); ciz();
    });
    tuval.addEventListener('pointermove', function (e) {
      if (!simdiki) return;
      e.preventDefault();
      var l = e.getCoalescedEvents ? e.getCoalescedEvents() : [];
      (l.length ? l : [e]).forEach(function (x) { simdiki.push(nokta(x)); });
      ciz();
    });
    function bitir() { if (simdiki) { simdiki = null; if (s.degisti) s.degisti(); } }
    tuval.addEventListener('pointerup', bitir);
    tuval.addEventListener('pointercancel', bitir);
    tuval.addEventListener('lostpointercapture', bitir);
    temizle.addEventListener('click', function () { cizgiler = []; simdiki = null; ciz(); if (s.degisti) s.degisti(); });
    if (window.ResizeObserver) new ResizeObserver(boyutla).observe(tuval);
    else window.addEventListener('resize', boyutla);
    return {
      kok: kok, tuval: tuval, temizleDugmesi: temizle,
      bos: function () { return !cizgiler.some(function (n) { return n.length > 0; }); },
      temizle: function () { cizgiler = []; ciz(); },
      boyutla: boyutla,
      /* Base64 PNG (önek yok). Çok yoğun ekranda sınırı aşarsa 1x ölçekle yeniden üretilir. */
      png: function () {
        boyutla();
        var v = tuval.toDataURL('image/png').split(',')[1] || '';
        if (v.length * 3 / 4 > IMZA_SINIR && olcek > 1) {
          var eski = olcek, w = tuval.width, h = tuval.height, r = tuval.getBoundingClientRect();
          tuval.width = Math.round(r.width); tuval.height = Math.round(r.height); olcek = 1; ciz();
          v = tuval.toDataURL('image/png').split(',')[1] || '';
          tuval.width = w; tuval.height = h; olcek = eski; ciz();
        }
        return v;
      }
    };
  }

  /* ---------- Teslim sepeti (Yusuf 2026-11-26: "teslim edilirken de teslim edilen ürünler zincirlenebilsin karekod
     zinciri ile yani tek karekod ile de teslim edilebilsin, birden fazla karekod da tek teslimatta yapılabilsin") ----------
     TKD.teslimSor(s) artık teslim SEPETİNİ açar. s: {fis, karekod?, ek?, oturum, api(metot, yol, govde), kamera?, yerelOku?}.
     (ek: baştan sepete konacak diğer fiş no'ları, ör. listede toplu seçilenler.)
     - Açılışta s.fis (s.karekod verilmişse o okunan metin) sepete eklenir; bağlı fişler kendiliğinden gelir, varsayılan
       hepsi seçili, tek tek çıkarılabilir. "Karekod okut / no yaz" alanına tabanca ya da klavyeyle başka fişler eklenir
       (bağlı olmasalar da). Telefonda s.kamera ise kamerayla okutulur (Android uygulamasında s.yerelOku, yoksa
       BarcodeDetector). Çift okutma yok sayılır; teslim edilmiş/iptal fiş eklenmez; hazır olmayan fiş uyarıyla görünür
       ama teslim edilmez.
     - FİŞLİ / FİŞSİZ: müşteri nüshasının karekodu okutulan fiş "fişli"dir; numarası yazılan ya da bağlı zincirden
       gelen fiş "fişsiz"dir (karar: Yusuf gözetimsiz, denetçinin önerisi). Elle "fişli" işaretleme yolu YOKTUR; sunucu
       okunan metni (karekod) yeniden doğrular. Sepette fişsiz fiş varsa fişsiz teslim yetkisi, her müşteri için
       telefonun son 4 hanesi (aynı telefonlu fişlerde bir kez) ve imza zorunludur.
     - Tek teslimat: sepetin tamamı tek teslim alan bilgisi, tek karekod onayı (POST /api/teslim/sepet/onay-ac; sepet
       değişirse karekod yenilenir) ya da tek ekran imzasıyla POST /api/teslim/sepet'e gider. Ya hepsi ya hiçbiri.
       Bitince her fişin teslim satırı ve ortak teslim no (teslim_id) gösterilir.
     Sonuç: sunucunun 200 yanıtı ({tamam, teslim_id, fisler, birlikte_teslim, teslim_kaydi}) ya da vazgeçilirse null.
     Eski 409 bagli_teslim sorusu (POST /api/fis/<no>/teslim-al) arayüzde artık kullanılmaz: bağlı fişler sepete gelir. */
  var QR_ARALIK_MS = 2000;
  function sureMetni(sn) { sn = Math.max(0, Math.floor(sn)); return Math.floor(sn / 60) + ':' + ('0' + (sn % 60)).slice(-2); }
  /* Açık sepet (masaüstünde tabanca okutması buraya yönlendirilir): {ekle(metin)} ya da null. */
  var SEPET_ACIK = null;
  function sepetAcik() { return SEPET_ACIK; }
  var KANIT_AD = { takip: 'müşteri nüshası okutuldu', 'fis-karekodu': 'fiş karekodu okutuldu' };
  var FISSIZ_NEDEN = { 'karekod-kanitsiz': 'karekodda kanıt yok (eski basılı fiş)', 'fis-karekodu': 'terzihane nüshası okutuldu' };
  /* Karekod onayı bu kurulumda kapalı (müşteri takip/bulut ayarı yok): ilk 409 qr_kapali yanıtından sonra sayfa
     yenilenene kadar karekod seçeneği kapalı kalır, sunucuya yeniden sorulmaz. */
  var QR_KAPALI = false;
  function teslimSor(s) {
    var f = s.fis || {}, fissizYetki = yetkiVar(s.oturum, 'fissiz-teslim');
    return new Promise(function (bitti) {
      var bas = kimlik('tsp'), onceki = document.activeElement, sonuc = null, bitmis = false, mesgul = false, tamamlanan = null;
      var satirlar = [], secim = {}, kaynak = {}, son = {}, s4 = {}, istekte = 0;
      /* --- sepet bölümü --- */
      var gir = el('input', { id: bas + '-gir', autocomplete: 'off', spellcheck: 'false', autocapitalize: 'off', sinif: 'mono',
        enterkeyhint: 'done', 'aria-describedby': bas + '-gira' });
      var ekleD = el('button', { type: 'button', sinif: 'dugme', metin: 'Ekle' });
      var kameraVar = !!s.kamera && (!!s.yerelOku || ('BarcodeDetector' in window && navigator.mediaDevices && navigator.mediaDevices.getUserMedia));
      var kameraD = kameraVar ? el('button', { type: 'button', sinif: 'dugme', metin: 'Kamerayla okut' }) : null;
      var kameraKap = el('div', { sinif: 'tk-sepet-kamera', hidden: true });
      var ekleDurum = el('p', { sinif: 'mut kucuk tk-sepet-durum', role: 'status', 'aria-live': 'polite' });
      var govdeT = el('tbody', { role: 'rowgroup' });
      /* Dar ekranda tr/td display:grid olur; bazı ekran okuyucular o zaman tablo anlamını bırakır: roller açıkça verilir. */
      var tablo = el('table', { sinif: 'tk-sepet-tablo', role: 'table' },
        el('caption', { sinif: 'sr', metin: 'Teslim sepetindeki fişler' }),
        el('thead', { role: 'rowgroup' }, el('tr', { role: 'row' }, ['Seç', 'Fiş', 'Müşteri', 'Ürün', 'Durum', 'Fiş karekodu', ''].map(function (b, i) {
          return el('th', { scope: 'col', role: 'columnheader', metin: b, 'aria-label': i === 6 ? 'Çıkar' : null });
        }))), govdeT);
      var bosNot = el('p', { sinif: 'mut', metin: 'Sepet boş. Fiş karekodunu okutun ya da fiş numarasını yazın.' });
      var toplam = el('p', { sinif: 'tk-sepet-toplam', role: 'status', 'aria-live': 'polite' });
      var uyariKap = el('ul', { sinif: 'tk-sepet-uyari', hidden: true });
      var fissizNot = el('p', { sinif: 'tk-sepet-fissiz', hidden: true });
      /* --- teslim alan bölümü --- */
      var yQr = el('input', { type: 'radio', name: bas + '-yon', value: 'qr' });
      var yEkran = el('input', { type: 'radio', name: bas + '-yon', value: 'ekran' });
      yQr.checked = !QR_KAPALI; yEkran.checked = QR_KAPALI; yQr.disabled = QR_KAPALI;
      var ad = el('input', { id: bas + '-ad', maxlength: '80', autocomplete: 'off', spellcheck: 'false' });
      ad.value = f.musteri || '';
      var alanAd = el('div', { sinif: 'alan' }, el('label', { for: bas + '-ad', metin: 'Teslim alan (ad soyad)' }), ad);
      var iliski = el('select', { id: bas + '-il' }, ILISKI.map(function (x) { return el('option', { value: x[0], metin: x[1] }); }));
      iliski.value = 'kendisi';
      var son4Kap = el('div', { sinif: 'tk-sepet-son4', hidden: true });
      var mesaj = el('div', { sinif: 'mesaj hata kucuk', role: 'alert', hidden: true });
      var imzaAl = el('input', { type: 'checkbox' });
      var imzaAlSatir = el('label', { sinif: 'onay' }, imzaAl, el('span', { metin: 'İmza al (isteğe bağlı)' }));
      var imza = imzaAlani({ degisti: function () { mesaj.hidden = true; }, alt: 'İmza bir kez alınır ve sepetteki bütün fişlerin teslim kaydına eklenir.' });
      var imzaBas = el('div', { sinif: 'tk-imza-bas', metin: 'Teslim alanın imzası' });
      var imzaKutu = el('div', { sinif: 'tk-imza-kap' }, imzaBas, imza.kok);
      var qrResim = el('div', { sinif: 'tk-qr-resim', role: 'img', 'aria-label': 'Teslim onayı karekodu (bütün sepet için)' });
      var qrDurum = el('div', { sinif: 'tk-qr-durum', role: 'status', 'aria-live': 'polite', metin: 'Karekod hazırlanıyor…' });
      var qrSure = el('span', { sinif: 'tk-qr-sure mono', 'aria-label': 'Kalan süre' });
      var qrBilgi = el('dl', { sinif: 'tk-teslim-kaydi tk-qr-bilgi', hidden: true });
      var qrImzaNotu = el('p', { sinif: 'tk-qr-imza-notu', hidden: true, metin: 'Sepette fişsiz fiş var: müşteri açılan sayfada kendi telefonunda parmağıyla imzalayacak (imza zorunlu).' });
      var qrYeni = el('button', { type: 'button', sinif: 'dugme kucuk', metin: 'Yeni karekod' });
      var qrEkran = el('button', { type: 'button', sinif: 'dugme kucuk', metin: 'Telefonu yok: ekranda imza' });
      var qrPanel = el('div', { sinif: 'tk-qr' }, qrResim,
        el('div', { sinif: 'tk-qr-yan' },
          el('p', { sinif: 'tk-qr-baslik', metin: 'Müşteri kendi telefonuyla okutsun' }),
          el('p', { sinif: 'mut kucuk', metin: 'Tek karekod bütün sepet içindir: sayfada toplam ürün sayısı görünür. Müşteri adını, telefonunu yazıp "Ürünlerimi eksiksiz teslim aldım" kutusunu işaretler. Bilgiler mağazaya şifreli gelir; onayın gelmesi 1 dakikaya kadar sürebilir. Yazılan ad ve telefon müşterinin beyanıdır, doğrulanmaz.' }),
          qrImzaNotu, qrDurum, qrBilgi,
          el('div', { sinif: 'dz-sira tk-qr-alt' }, qrSure, qrYeni, qrEkran)));
      var gonderD = el('button', { type: 'submit', sinif: 'dugme sonraki', metin: 'Teslim et' });
      var vazgec = el('button', { type: 'button', sinif: 'dugme', metin: 'Vazgeç' });
      var teslimBolum = el('div', { sinif: 'tk-sepet-teslim' },
        el('fieldset', { sinif: 'tk-teslim-tur' }, el('legend', { metin: 'Teslim onayı (bütün sepet için bir kez)' }),
          el('label', { sinif: 'onay' }, yQr, el('span', { metin: QR_KAPALI ? 'Karekodla onay (bu kurulumda kapalı: müşteri takip ayarı yok)' : 'Karekodla onay' })),
          el('label', { sinif: 'onay' }, yEkran, el('span', { metin: 'Telefonu yok: ekranda imza' }))),
        qrPanel,
        el('div', { sinif: 'dz-izgara' }, alanAd,
          el('div', { sinif: 'alan' }, el('label', { for: bas + '-il', metin: 'Müşteriye yakınlığı' }), iliski)),
        son4Kap, imzaAlSatir, imzaKutu);
      var form = el('form', { method: 'dialog', novalidate: true },
        el('h2', { id: bas, metin: 'Teslim sepeti' }),
        el('p', { sinif: 'mut', metin: 'Fiş karekodlarını sırayla okutun ya da numara yazın; bağlı fişler kendiliğinden gelir. Hepsi tek seferde teslim edilir.' }),
        el('div', { sinif: 'tk-sepet-ekle' },
          el('label', { for: bas + '-gir', metin: 'Karekod okut / no yaz' }),
          el('div', { sinif: 'tk-sepet-ekle-sira' }, gir, ekleD, kameraD),
          el('span', { sinif: 'mut kucuk', id: bas + '-gira', metin: 'Müşteri nüshasının karekodu okutulan fiş “fişli” sayılır. Numarası elle yazılan, bağlı fiş olarak kendiliğinden gelen ya da karekodunda kanıt parçası olmayan (eski basılı) fiş “fişsiz” sayılır; fişli işareti elle konamaz. En çok 30 fiş.' })),
        kameraKap, ekleDurum, bosNot, el('div', { sinif: 'tk-sepet-kaydir' }, tablo), toplam, uyariKap, fissizNot,
        teslimBolum, mesaj,
        el('div', { sinif: 'dz-sira' }, vazgec, gonderD));
      var kutu = el('dialog', { sinif: 'dz-kutu tk-teslim tk-sepet', 'aria-labelledby': bas }, form);

      /* --- sepet durumu --- */
      function secililer() { return satirlar.filter(function (x) { return x.hazir && secim[x.no] !== false; }); }
      function satirGovde(x) { return { no: x.no, okutuldu: !!x.okutuldu, karekod: x.karekod || '' }; }
      function fissizVar() { return secililer().some(function (x) { return !x.okutuldu; }); }
      function sepetAnahtari() { return secililer().map(function (x) { return x.no + (x.okutuldu ? '+' : '-'); }).sort().join(','); }
      function gruplar() {
        var sec = {}; secililer().forEach(function (x) { if (!x.okutuldu) sec[x.no] = 1; });
        return (son.son4_gruplari || []).map(function (g) {
          var l = (g.fisler || []).filter(function (n) { return sec[n]; });
          return l.length ? { anahtar: l[0], fisler: l, musteri: g.musteri || '' } : null;
        }).filter(Boolean);
      }
      function durumYaz(t, tur) { ekleDurum.textContent = t || ''; ekleDurum.className = 'kucuk tk-sepet-durum ' + (tur === 'hata' ? 'dz-neden' : tur === 'tamam' ? 'tk-sepet-tamam' : 'mut'); }
      /* Sepet istekleri SIRAYLA gider (denetçi bulgusu: art arda okutmada ikinci istek birinciyi içermeyen listeyle
         gidip yanıtı birinci fişi siliyordu). Her iş kuyruktaki bir öncekinin yanıtı işlendikten sonra çalışır; liste
         değişikliği (çıkarma, toplu ekleme) de kuyrukta, istek kurulmadan hemen önce yapılır. */
      var kuyruk = Promise.resolve();
      function sirala(is) {
        istekte++; goster();
        var p = kuyruk.then(function () { return bitmis ? null : is(); });
        kuyruk = p.then(function () { istekte--; goster(); }, function () { istekte--; goster(); });
        return p;
      }
      function denetle(ekle, once) { return sirala(function () { if (once) once(); return denetleIc(ekle); }); }
      function denetleIc(ekle) {
        var gonder = { fisler: satirlar.map(satirGovde) };
        if (ekle) gonder.ekle = ekle;
        return s.api('POST', '/api/teslim/sepet/denetle', gonder).then(function (r) {
          if (bitmis) return;
          son = r || {}; satirlar = son.fisler || [];
          (son.eklenen || []).forEach(function (n, i) { secim[n] = true; if (!kaynak[n]) kaynak[n] = i === 0 ? 'elle' : 'zincir'; });
          if (ekle) {
            if (son.ekle_hatasi) durumYaz(son.ekle_hatasi, 'hata');
            else if (son.zaten_sepette && !(son.eklenen || []).length) durumYaz('Bu fiş zaten sepette; ikinci okutma yok sayıldı.' + (son.kanit_notu ? ' ' + son.kanit_notu : ''), '');
            else if ((son.eklenen || []).length) {
              var ilk = son.eklenen[0], z = son.eklenen.slice(1);
              durumYaz(ilk + ' sepete eklendi' + (z.length ? '; bağlı fişler de geldi: ' + z.join(', ') : '') + '.' + (son.kanit_notu ? ' ' + son.kanit_notu : ''), 'tamam');
            } else durumYaz(son.kanit_notu || 'Sepet güncellendi.', '');
          }
          ciz(); sepetDegisti();
        }, function (e) {
          if (bitmis) return;
          durumYaz('Sepet güncellenemedi: ' + hataIleti(e), 'hata'); goster();
        });
      }
      function ekleIste(v) {
        if (son.dolu) { durumYaz('Sepet dolu: bir teslimatta en çok ' + (son.en_cok || 30) + ' fiş. Önce bu sepeti teslim edin.', 'hata'); return; }
        denetle(v);
      }
      function ekleGir() {
        var v = gir.value.trim();
        if (!v) { durumYaz('Karekodu okutun ya da fiş numarasını yazın.', 'hata'); try { gir.focus(); } catch (e) {} return; }
        gir.value = ''; ekleIste(v);
      }
      function cikar(no) {
        durumYaz(no + ' sepetten çıkarılıyor…', '');
        denetle(null, function () {
          satirlar = satirlar.filter(function (x) { return x.no !== no; }); delete secim[no];
        }).then(function () { if (!bitmis) durumYaz(no + ' sepetten çıkarıldı.', ''); });
      }
      function hucre(m, sinif, etiket) { return el('td', { sinif: sinif || null, role: 'cell', 'data-etiket': etiket || null }, m); }
      function ciz() {
        govdeT.textContent = '';
        satirlar.forEach(function (x) {
          var c = el('input', { type: 'checkbox', 'aria-label': x.no + ' teslim edilsin', disabled: !x.hazir });
          c.checked = !!x.hazir && secim[x.no] !== false;
          c.addEventListener('change', function () { secim[x.no] = c.checked; ciz(); sepetDegisti(); });
          var kanit = x.okutuldu
            ? el('span', { sinif: 'tk-sepet-fisli' }, 'Okutuldu · fişli', el('span', { sinif: 'mut kucuk tk-blok', metin: KANIT_AD[x.kanit] || '' }))
            : el('span', { sinif: 'tk-sepet-fissiz-et' }, 'Okutulmadı · fişsiz', el('span', { sinif: 'mut kucuk tk-blok', metin: FISSIZ_NEDEN[x.kanit] || (kaynak[x.no] === 'zincir' ? 'bağlı fiş olarak geldi' : 'numara yazıldı') }));
          var durum = x.hazir ? el('span', { metin: DURUM_AD[x.durum] || x.durum })
            : el('span', { sinif: 'dz-neden', metin: 'Hazır değil (' + (DURUM_AD[x.durum] || x.durum) + '): teslim edilmez' });
          var cik = el('button', { type: 'button', sinif: 'dugme kucuk', metin: 'Çıkar', 'aria-label': x.no + ' fişini sepetten çıkar', tikla: function () { cikar(x.no); } });
          govdeT.appendChild(el('tr', { role: 'row', sinif: x.hazir ? (c.checked ? '' : 'tk-sepet-secilmedi') : 'dz-pasif' },
            hucre(el('label', { sinif: 'tk-sepet-sec' }, c)), hucre(el('b', { sinif: 'mono', metin: x.no })), hucre(x.musteri || '', null, 'Müşteri'),
            hucre(String(x.urun_sayisi || 0), 'mono', 'Ürün'), hucre(durum, null, 'Durum'), hucre(kanit, null, 'Fiş karekodu'), hucre(cik)));
        });
        bosNot.hidden = satirlar.length > 0;
        tablo.hidden = !satirlar.length;
        var sec = secililer(), urun = sec.reduce(function (t, x) { return t + (x.urun_sayisi || 0); }, 0);
        toplam.textContent = sec.length ? 'Teslim edilecek: ' + sec.length + ' fiş · toplam ' + urun + ' ürün' : (satirlar.length ? 'Teslim edilecek hazır fiş seçilmedi.' : '');
        uyariKap.textContent = '';
        (son.uyarilar || []).forEach(function (u) { uyariKap.appendChild(el('li', { metin: u })); });
        uyariKap.hidden = !uyariKap.childNodes.length;
        var fz = fissizVar();
        fissizNot.hidden = !fz;
        fissizNot.textContent = fz ? 'Sepette fiş karekodu okutulmamış (fişsiz) fiş var: ' + (fissizYetki
          ? 'teslim alanın imzası ve her müşteri için telefonun son 4 hanesi zorunlu.'
          : 'bu cihazın fişsiz teslim yetkisi yok. Müşteri nüshasının karekodunu okutun ya da o fişleri sepetten çıkarın (mağaza bilgisayarından teslim edilebilir).') : '';
        son4Ciz();
        goster();
      }
      function son4Ciz() {
        var g = gruplar(), anahtar = g.map(function (x) { return x.fisler.join('+'); }).join(',');
        son4Kap.hidden = !g.length || !fissizYetki;
        if (son4Kap._anahtar === anahtar) return;
        son4Kap._anahtar = anahtar;
        son4Kap.textContent = '';
        if (!g.length) return;
        son4Kap.appendChild(el('p', { sinif: 'tk-imza-bas', metin: 'Fişsiz fişler: müşteri telefonunun son 4 hanesi' }));
        g.forEach(function (x) {
          var id = kimlik('s4');
          var gi = el('input', { id: id, inputmode: 'numeric', autocomplete: 'off', maxlength: '4', pattern: '[0-9]{4}', sinif: 'mono', 'data-anahtar': x.anahtar, 'aria-describedby': id + 'a' });
          gi.value = s4[x.anahtar] || '';
          gi.addEventListener('input', function () { var t = gi.value.replace(/\D/g, '').slice(0, 4); if (t !== gi.value) gi.value = t; s4[x.anahtar] = t; mesaj.hidden = true; });
          son4Kap.appendChild(el('div', { sinif: 'alan tk-son4' },
            el('label', { for: id, metin: (x.musteri || 'Müşteri') + ' · ' + x.fisler.join(', ') }), gi,
            el('span', { sinif: 'mut kucuk', id: id + 'a', metin: 'Teslim alana sorun; ekranda gösterilmez, kaydedilmez.' })));
        });
      }
      /* --- karekod (bütün sepet) --- */
      var qr = { t: null, bitis: 0, onay: null, sorgu: null, sayac: null, istekte: false, ac: false, imzaGerekli: false, anahtar: '', fisler: null };
      function qrImzali() { return !!(qr.onay && qr.onay.imzali); }
      function karekod() { return yQr.checked; }
      function qrDurdur() { if (qr.sorgu) clearInterval(qr.sorgu); if (qr.sayac) clearInterval(qr.sayac); qr.sorgu = qr.sayac = null; }
      function telefonFarkli() { return !!(qr.onay && (qr.onay.fis_telefonu === 'farkli' || qr.onay.fis_telefonu === 'kismen')); }
      function qrBilgiCiz(o) {
        qrBilgi.textContent = '';
        var fisTel = o.fis_telefonu === 'ayni' ? 'Aynı' : o.fis_telefonu === 'farkli' ? 'Farklı: teslim alanın yakınlığını seçin' : o.fis_telefonu === 'kismen' ? 'Bazı fişlerde farklı: yakınlığı seçin' : '';
        var imzaMetni = o.imzali ? 'Müşteri telefonunda imzaladı' : (fissizVar() ? 'Gelmedi: aşağıda ekranda imzalatın' : '');
        [['Teslim alan', o.ad], ['Telefon', o.tel_maskeli], ['Fişteki numarayla', fisTel], ['Okutan cihaz', o.cihaz], ['Onay saati', o.zaman_metni], ['İmza', imzaMetni]].forEach(function (x) {
          if (x[1]) qrBilgi.appendChild(el('div', { sinif: x[0] === 'Fişteki numarayla' && o.fis_telefonu !== 'ayni' ? 'tk-uyari' : '' }, el('dt', { metin: x[0] }), el('dd', { sinif: x[0] === 'Telefon' || x[0] === 'Onay saati' ? 'mono' : '', metin: x[1] })));
        });
        qrBilgi.hidden = false;
        if (telefonFarkli() && iliski.value === 'kendisi') iliski.value = 'yakini';
      }
      function sayacCiz() { var kalan = (qr.bitis - Date.now()) / 1000; qrSure.textContent = qr.t ? sureMetni(kalan) : ''; if (qr.t && kalan <= 0) qrSuresiDoldu(); }
      function qrSuresiDoldu() {
        qrDurdur(); qr.t = null; qr.onay = null; qrBilgi.hidden = true;
        qrResim.classList.add('tk-qr-soluk');
        qrDurum.textContent = 'Karekodun süresi doldu. "Yeni karekod" ile yenileyin.';
        qrSure.textContent = '0:00'; goster();
      }
      function qrSor() {
        if (!qr.t || qr.istekte || qr.onay) return;
        var t = qr.t; qr.istekte = true;
        s.api('POST', '/api/teslim/sepet/onay-durum', { fisler: qr.fisler, t: t }).then(function (r) {
          qr.istekte = false;
          if (bitmis || t !== qr.t) return;
          if (typeof r.kalan_sn === 'number') qr.bitis = Date.now() + r.kalan_sn * 1000;
          if (r.durum === 'onaylandi') {
            qr.onay = r; if (qr.sorgu) clearInterval(qr.sorgu); qr.sorgu = null;
            qrDurum.textContent = 'Müşteri onayladı. Bilgileri kontrol edip teslim edin.';
            qrDurum.className = 'tk-qr-durum tk-qr-tamam';
            qrBilgiCiz(r); goster();
            try { gonderD.focus(); } catch (e) {}
          } else {
            qrDurum.textContent = r.saat_sorunu ? 'Onay okunamıyor: bu cihazın saati yanlış görünüyor. Saat ayarını denetleyin.'
              : r.baglanti_sorunu ? 'Onay bekleniyor… (internet bağlantısı yavaş ya da yok; denemeye devam ediliyor)' : 'Onay bekleniyor…';
          }
        }, function (e) {
          qr.istekte = false;
          if (bitmis || t !== qr.t) return;
          var v = (e && e.veri) || {};
          if (e && e.kod === 410) { qrSuresiDoldu(); return; }
          if (e && (e.kod === 404 || e.kod === 422 || e.kod === 409)) {
            qrDurdur(); qr.t = null;
            qrDurum.textContent = (v.ileti || v.hata || 'Karekod geçersiz') + ' "Yeni karekod" ile yeniden deneyin.';
            goster(); return;
          }
          qrDurum.textContent = 'Onay bekleniyor… (' + hataIleti(e) + ')';
        });
      }
      function qrAc() {
        qrDurdur(); qr.t = null; qr.onay = null; qr.anahtar = sepetAnahtari();
        qrBilgi.hidden = true; qrResim.textContent = ''; qrResim.classList.remove('tk-qr-soluk');
        qrDurum.className = 'tk-qr-durum'; qrSure.textContent = '';
        if (!secililer().length) { qr.ac = false; qrDurum.textContent = 'Karekod, sepete hazır fiş eklenince hazırlanır.'; goster(); return; }
        qr.ac = true; qrDurum.textContent = 'Karekod hazırlanıyor…'; qrYeni.disabled = true; goster();
        var fisler = secililer().map(satirGovde), anahtar = qr.anahtar;
        s.api('POST', '/api/teslim/sepet/onay-ac', { fisler: fisler }).then(function (r) {
          qrYeni.disabled = false;
          if (bitmis || anahtar !== qr.anahtar) return;
          qr.t = r.t; qr.fisler = fisler; qr.bitis = Date.now() + (r.kalan_sn || 900) * 1000;
          qr.imzaGerekli = !!r.imza_gerekli;
          // SVG sunucunun kendi ürettiği karekoddur (yalnız <svg><path>); dış içerik değildir.
          qrResim.innerHTML = r.qr_svg || '';
          qrDurum.textContent = 'Onay bekleniyor… (' + (r.toplam_urun || '') + ' ürün)';
          qr.sorgu = setInterval(qrSor, QR_ARALIK_MS);
          qr.sayac = setInterval(sayacCiz, 1000);
          sayacCiz(); goster();
        }, function (e) {
          qrYeni.disabled = false; qr.ac = false;
          if (bitmis) return;
          var v = (e && e.veri) || {};
          if (v.hata === 'qr_kapali') {
            // Kurulumda karekod onayı yok: bu sayfada bir daha sorulmaz.
            QR_KAPALI = true; yQr.disabled = true; yEkran.checked = true;
            qrDurum.textContent = v.ileti || 'Karekodla onay bu kurulumda kapalı.';
            goster(); return;
          }
          if (v.hata === 'qr_cok_urun' || v.hata === 'bulut_yok' || v.hata === 'saat_farki') {
            yEkran.checked = true; goster(); hata(v.ileti || hataIleti(e)); return;
          }
          if (v.hata === 'sepet_cok_buyuk') { qrDurum.textContent = v.ileti || 'Sepet çok büyük.'; goster(); return; }
          qrDurum.textContent = 'Karekod hazırlanamadı: ' + hataIleti(e);
          goster();
        });
      }
      /* Sepet (seçim ya da fişli/fişsiz) değişti: açık karekod eski sepete aittir; yenisi hazırlanır. */
      function sepetDegisti() {
        if (!karekod()) { goster(); return; }
        var a = sepetAnahtari();
        if (a === qr.anahtar && (qr.t || qr.ac)) { goster(); return; }
        if (qr.onay) hata('Sepet değişti: önceki karekod onayı bu sepet için geçerli değil, yeni karekod hazırlandı.');
        qrAc();
      }
      function goster() {
        var sec = secililer().length, fz = fissizVar(), k = karekod();
        teslimBolum.hidden = !sec;
        qrPanel.hidden = !k;
        alanAd.hidden = k;
        qrImzaNotu.hidden = !(k && fz);
        imzaAlSatir.hidden = fz || k;
        var qrEkranImza = k && fz && !!qr.onay && !qrImzali();
        imzaKutu.hidden = k ? !qrEkranImza : !(fz || imzaAl.checked);
        imzaBas.textContent = qrEkranImza ? 'Müşterinin imzası karekoddan gelmedi: ekranda imzalatın (zorunlu)'
          : fz ? 'Teslim alanın imzası (zorunlu: sepette fişsiz fiş var)' : 'Teslim alanın imzası';
        gonderD.disabled = mesgul || !!tamamlanan || !sec || istekte > 0 || (k && !qr.onay) || (fz && !fissizYetki);
        gonderD.textContent = mesgul ? 'Gönderiliyor…' : sec ? 'Teslim et (' + sec + ' fiş)' : 'Teslim et';
        if (!imzaKutu.hidden) imza.boyutla();
      }
      imzaAl.addEventListener('change', function () { mesaj.hidden = true; goster(); });
      [yQr, yEkran].forEach(function (x) { x.addEventListener('change', function () {
        mesaj.hidden = true;
        if (karekod() && !qr.t && !qr.ac) qrAc(); else goster();
      }); });
      qrYeni.addEventListener('click', function () { mesaj.hidden = true; qrAc(); });
      qrEkran.addEventListener('click', function () { yEkran.checked = true; mesaj.hidden = true; goster(); try { ad.focus(); } catch (e) {} });
      ekleD.addEventListener('click', ekleGir);
      gir.addEventListener('keydown', function (e) { if (e.key === 'Enter') { e.preventDefault(); ekleGir(); } });
      /* --- kamera (telefon) --- */
      var kamera = { akis: null, zaman: 0, tarayici: null, son: '', sonZaman: 0 };
      function kameraKapat() {
        clearTimeout(kamera.zaman);
        if (kamera.akis) { kamera.akis.getTracks().forEach(function (t) { t.stop(); }); kamera.akis = null; }
        kameraKap.textContent = ''; kameraKap.hidden = true;
        if (kameraD) kameraD.textContent = 'Kamerayla okut';
      }
      function okundu(ham) {
        ham = String(ham || '').trim();
        var simdi = Date.now();
        if (!ham || (ham === kamera.son && simdi - kamera.sonZaman < 3000)) return;
        kamera.son = ham; kamera.sonZaman = simdi;
        if (navigator.vibrate) try { navigator.vibrate(60); } catch (e) {}
        ekleIste(ham);
      }
      function kameraAc() {
        if (s.yerelOku) { s.yerelOku(okundu); return; }
        if (kamera.akis) { kameraKapat(); return; }
        try { kamera.tarayici = kamera.tarayici || new window.BarcodeDetector({ formats: ['qr_code'] }); } catch (e) { durumYaz('Kamera okuyucu kullanılamıyor; numarayı yazın.', 'hata'); return; }
        navigator.mediaDevices.getUserMedia({ video: { facingMode: 'environment' }, audio: false }).then(function (akis) {
          if (bitmis) { akis.getTracks().forEach(function (t) { t.stop(); }); return; }
          kamera.akis = akis;
          var v = el('video', { playsinline: true, muted: true, 'aria-label': 'Kamera görüntüsü' });
          v.srcObject = akis; kameraKap.textContent = ''; kameraKap.appendChild(v);
          kameraKap.appendChild(el('p', { sinif: 'mut kucuk', metin: 'Karekodları sırayla tutun; her okunan fiş sepete eklenir.' }));
          kameraKap.hidden = false; kameraD.textContent = 'Kamerayı kapat';
          v.play().catch(function () {});
          (function tara() {
            if (!kamera.akis) return;
            var sonra = function () { kamera.zaman = setTimeout(tara, 300); };
            if (v.readyState < 2) { sonra(); return; }
            kamera.tarayici.detect(v).then(function (k) { if (k && k.length) okundu(k[0].rawValue); sonra(); }, sonra);
          })();
        }, function () { durumYaz('Kamera izni verilmedi. İzin verip yeniden deneyin ya da numarayı yazın.', 'hata'); });
      }
      if (kameraD) kameraD.addEventListener('click', kameraAc);
      /* --- teslim --- */
      function hata(t, odak) { mesaj.textContent = t; mesaj.hidden = false; if (odak) try { odak.focus(); } catch (e) {} }
      function kapat(r) { sonuc = r; if (kutu.open && kutu.close) kutu.close(); else bitir(); }
      function bitir() {
        if (bitmis) return; bitmis = true;
        qrDurdur(); kameraKapat();
        if (SEPET_ACIK === tutamak) SEPET_ACIK = null;
        if (kutu.parentNode) kutu.parentNode.removeChild(kutu);
        if (onceki && onceki.focus) try { onceki.focus(); } catch (e) {}
        bitti(sonuc);
      }
      function son4Girdisi(anahtar) { return son4Kap.querySelector('input[data-anahtar="' + anahtar + '"]'); }
      function sonucCiz(r) {
        tamamlanan = r; qrDurdur(); kameraKapat();
        if (SEPET_ACIK === tutamak) SEPET_ACIK = null;
        form.textContent = '';
        var liste = el('ul', { sinif: 'dz-liste tk-sepet-sonuc' }, (r.fisler || []).map(function (x) {
          return el('li', {}, el('span', {}, el('b', { sinif: 'mono', metin: x.no }), ' · ' + (x.musteri || '') + ' · ' + (x.urun_sayisi || 0) + ' ürün · ' +
            (x.fisli ? 'Fişli' : 'Fişsiz (son 4 + imza)') + (x.ana_fis ? ' · birlikte: ' + x.ana_fis : ''),
            el('span', { sinif: 'mut kucuk tk-blok mono', metin: 'Teslim · ' + String(x.tarih || '').split('-').reverse().join('.') + ' ' + String(x.saat || '').slice(0, 5) + ' · ' + (x.teslim_id || '') })));
        }));
        var kapatD = el('button', { type: 'button', sinif: 'dugme sonraki', metin: 'Kapat', tikla: function () { kapat(r); } });
        form.appendChild(el('h2', { id: bas, metin: 'Teslim edildi' }));
        form.appendChild(el('p', { role: 'status', sinif: 'tk-sepet-tamam' }, (r.fisler || []).length + ' fiş tek teslimatta teslim edildi. Teslim no: ', el('b', { sinif: 'mono tk-sepet-tid', metin: r.teslim_id || '' })));
        form.appendChild(liste);
        form.appendChild(el('p', { sinif: 'mut kucuk', metin: 'Teslim alan bilgisi şifreli kaydedildi; fiş ayrıntısında maskeli görünür.' + (r.teslim_kaydi && r.teslim_kaydi.imzali ? ' İmza bir kez alındı, bütün fişlerin kaydına eklendi.' : '') }));
        form.appendChild(el('div', { sinif: 'dz-sira' }, kapatD));
        try { kapatD.focus(); } catch (e) {}
      }
      function gonder(govde) {
        mesgul = true; vazgec.disabled = true; goster();
        return s.api('POST', '/api/teslim/sepet', govde).then(function (r) {
          mesgul = false; vazgec.disabled = false;
          sonucCiz(r || { tamam: true });
        }, function (e) {
          mesgul = false; vazgec.disabled = false; goster();
          var v = (e && e.veri) || {};
          if (v.hata === 'hazir_degil' || v.hata === 'sepet_gecersiz') { hata((v.ileti || hataIleti(e)) + ' Sepet yenilendi.'); denetle(); return; }
          if (v.hata === 'sepet_cok_buyuk') { hata(v.ileti || 'Bir teslimatta en çok 30 fiş teslim edilir; sepeti bölün.'); return; }
          if (v.hata === 'son4_yanlis' || v.hata === 'son4_gerekli') {
            var gi = son4Girdisi(v.fis) || son4Kap.querySelector('input');
            if (v.hata === 'son4_yanlis' && gi) { gi.value = ''; s4[v.fis] = ''; }
            hata(v.ileti || 'Telefonun son 4 hanesi gerekli.', gi); return;
          }
          if (v.hata === 'son4_kilitli') { hata(v.ileti || 'Çok fazla yanlış deneme; bir süre sonra yeniden deneyin.'); return; }
          if (v.hata === 'iliski_gerekli') { if (iliski.value === 'kendisi') iliski.value = 'yakini'; hata(v.ileti || 'Teslim alanın müşteriye yakınlığını seçin.', iliski); return; }
          if (v.hata === 'onay_yok') { qrSuresiDoldu(); hata(v.ileti || 'Karekod onayı bulunamadı; yeni karekod oluşturun.'); return; }
          if (e && e.kod === 413 && !v.hata) { hata('İmza çok büyük. Temizleyip daha sade imzalatın.'); return; }
          if (v.hata === 'imza_gerekli') { goster(); hata(v.ileti || 'Sepette fişsiz fiş var: imza zorunlu.', imza.temizleDugmesi); return; }
          if (v.hata === 'teslim_yazilamadi') { hata(v.ileti || 'Teslim kaydedilemedi; hiçbir fiş teslim edilmedi.'); return; }
          hata(hataIleti(e));
        });
      }
      form.addEventListener('submit', function (ev) {
        ev.preventDefault();
        if (mesgul || tamamlanan) return;
        mesaj.hidden = true;
        var sec = secililer();
        if (!sec.length) { hata('Sepette teslim edilecek hazır fiş yok.', gir); return; }
        var fz = fissizVar();
        if (fz && !fissizYetki) { hata('Sepette fişsiz fiş var ve bu cihazın fişsiz teslim yetkisi yok. Müşteri nüshasının karekodunu okutun ya da o fişleri çıkarın.'); return; }
        var govde = { fisler: sec.map(satirGovde), iliski: iliski.value };
        if (karekod()) {
          if (!qr.t || !qr.onay) { hata('Müşterinin karekodla onayı bekleniyor. Telefonu yoksa "ekranda imza" seçin.'); return; }
          if (qr.anahtar !== sepetAnahtari()) { hata('Sepet değişti; yeni karekod gerekli.'); qrAc(); return; }
          if (telefonFarkli() && iliski.value === 'kendisi') { hata('Karekodda yazılan telefon fişteki numaradan farklı. Teslim alanın müşteriye yakınlığını seçin (yakını ya da diğer).', iliski); return; }
          govde.onay_belirteci = qr.t;
          if (fz && !qrImzali()) {
            if (imza.bos()) { goster(); hata('Sepette fişsiz fiş var: imza zorunlu. Müşterinin imzası karekoddan gelmedi; ekranda imzalatın.', imza.temizleDugmesi); return; }
            var pq = imza.png();
            if (pq.length * 3 / 4 > IMZA_SINIR) { hata('İmza çok büyük. Temizleyip daha sade imzalatın.'); return; }
            govde.imza_png_base64 = pq;
          }
        } else {
          var a = ad.value.trim().replace(/\s+/g, ' ');
          if (a.split(' ').length < 2) { hata('Teslim alanın adını ve soyadını yazın.', ad); return; }
          if (a.length > 80) { hata('Ad soyad en çok 80 harf olabilir.', ad); return; }
          if (!AD_DESENI.test(a)) { hata('Ad soyadda yalnız harf, boşluk, tire, kesme işareti ve nokta olabilir.', ad); return; }
          govde.teslim_alan = a;
          if (fz || imzaAl.checked) {
            if (imza.bos()) { hata(fz ? 'Sepette fişsiz fiş var: imza zorunlu. Teslim alana dokunmatik ekranda ya da fareyle imzalatın.' : 'İmza alanı boş. İmzalatın ya da "İmza al" işaretini kaldırın.', imza.temizleDugmesi); return; }
            var p = imza.png();
            if (p.length * 3 / 4 > IMZA_SINIR) { hata('İmza çok büyük. Temizleyip daha sade imzalatın.'); return; }
            govde.imza_png_base64 = p;
          }
        }
        if (fz) {
          var s4g = {}, eksik = null;
          gruplar().forEach(function (g) { var d = s4[g.anahtar] || ''; if (!/^\d{4}$/.test(d) && !eksik) eksik = g; s4g[g.anahtar] = d; });
          if (eksik) { hata(eksik.fisler.join(', ') + ' için müşteri telefonunun son 4 hanesini yazın.', son4Girdisi(eksik.anahtar)); return; }
          govde.telefon_son4 = s4g;
        }
        gonder(govde);
      });
      vazgec.addEventListener('click', function () { kapat(tamamlanan); });
      kutu.addEventListener('close', bitir);
      kutu.addEventListener('cancel', function (e) { if (mesgul) { e.preventDefault(); return; } sonuc = tamamlanan; });
      var tutamak = { ekle: function (ham) { if (!tamamlanan) ekleIste(String(ham || '').trim()); } };
      SEPET_ACIK = tutamak;
      document.body.appendChild(kutu);
      if (kutu.showModal) kutu.showModal(); else kutu.setAttribute('open', '');
      ciz();
      var ilk = (s.karekod || f.no || '').trim();
      var ek = (s.ek || []).filter(function (n) { return n && n !== f.no; });
      if (ilk) denetle(ilk);
      // Toplu teslim: diğer seçili fişler de sepete (karekodları okutulmadı: fişsiz; okutulursa fişliye yükselir).
      if (ek.length) denetle(null, function () {
        ek.forEach(function (n) { if (!satirlar.some(function (x) { return x.no === n; })) { satirlar.push({ no: n, okutuldu: false }); kaynak[n] = 'elle'; } });
      });
      sirala(function () { if (karekod() && !qr.t && !qr.ac && !bitmis) qrAc(); });
      try { gir.focus(); } catch (e) {}
    });
  }

  /* Fiş ayrıntısında teslim kaydı (Yusuf 2026-11-26: "daha sonra maskeli olarak görünsün. yalnızca yöneticiler ve
     mağaza müdürleri görebilsin"). Varsayılan MASKELİ: ad "M*** Y***", telefon son 4 hane, okutan cihazın özeti; IP ve
     imza görüntüsü yok. "Açık göster" düğmesi yalnız t.acik_yetki ise (sunucu belirler; mağaza bilgisayarında PIN
     istenir) ve s.acikGetir(no) verilmişse görünür: GET /api/fis/<no>/teslim-acik tam ad, telefon, IP ve ham cihaz
     bilgisini verir; imza görüntüsü (s.imzaGetir, GET /api/fis/<no>/imza; sunucuda çözülür) ancak bundan sonra
     çekilir. Sunucu her açık görüntülemeyi kişisel veri olmadan denetim günlüğüne yazar. Açık hâl yalnız bu
     görüntülemede kalır: ACIK_SURE_MS (60 sn) sonra kendiliğinden, "Maskele" düğmesiyle, fiş değişince ya da yönetici
     oturumu kapanınca (TKD.teslimMaskele(), masaüstü kilitle/yoneticiKapandi çağırır) yeniden maskelenir. Açıkken
     20 sn'lik liste yenilemesi görünümü kapatmaz (süre dolana kadar). */
  var ACIK_SURE_MS = 60000;
  var ACIK_GORUNUMLER = [];
  function teslimMaskele() {
    var l = ACIK_GORUNUMLER.slice();
    ACIK_GORUNUMLER.length = 0;
    l.forEach(function (m) { try { m(); } catch (e) { /* görünüm kaldırılmış olabilir */ } });
  }
  var IMZA_YONTEM_AD = { qr_imza: 'Müşterinin telefonunda (karekod)', cihaz_imza: 'Mağaza ekranında' };
  function teslimKaydiCiz(kap, f, s) {
    if (!kap) return;
    var t = f && f.teslim_kaydi;
    // Açık görünüm açıkken (aynı fiş, aynı kayıt) yenileme onu kapatmasın.
    var anahtar = t ? f.no + ':' + (t.tarih || '') + ':' + (t.saat || '') + ':' + (t.imza_sha256 || '') : '';
    if (anahtar && kap._acik === anahtar) return;
    kap._acik = null;
    if (kap._acikSure) { clearTimeout(kap._acikSure); kap._acikSure = null; }
    kap.textContent = '';
    if (!t) return;
    var qrOnay = t.dogrulama === 'qr' || t.dogrulama === 'qr+telefon-son4';
    var adMetni = t.ad || t.teslim_alan || (t.sifreli ? 'Şifreli (yalnız yetkili cihazda görünür)' : '');
    var imzaMetni = t.imza_sha256 ? 'Var · ' + (IMZA_YONTEM_AD[t.imza_yontemi] || 'ekranda') : 'Yok';
    var dl = el('dl', { sinif: 'tk-teslim-kaydi' },
      el('div', {}, el('dt', { metin: 'Teslim alan' }), el('dd', { metin: adMetni })),
      (t.tel_maskeli ? el('div', {}, el('dt', { metin: 'Telefon' }), el('dd', { sinif: 'mono', metin: t.tel_maskeli })) : null),
      el('div', {}, el('dt', { metin: 'İlişki' }), el('dd', { metin: iliskiAd(t.iliski) })),
      el('div', {}, el('dt', { metin: 'Fiş' }), el('dd', { metin: t.fisli ? 'Fişli' : 'Fişsiz (telefon son 4 ile doğrulandı)' })),
      el('div', {}, el('dt', { metin: 'Onay' }), el('dd', { metin: qrOnay ? 'Karekodla, müşterinin telefonundan' : (t.imza_sha256 ? 'Ekranda imza' : 'İmzasız') })),
      el('div', {}, el('dt', { metin: 'İmza' }), el('dd', { metin: imzaMetni })),
      (t.cihaz ? el('div', {}, el('dt', { metin: 'Okutan cihaz' }), el('dd', { metin: t.cihaz })) : null),
      (t.ana_fis ? el('div', {}, el('dt', { metin: 'Birlikte teslim' }), el('dd', { sinif: 'mono', metin: t.ana_fis })) : null),
      (t.tarih ? el('div', {}, el('dt', { metin: 'Zaman' }), el('dd', { sinif: 'mono', metin: String(t.tarih).split('-').reverse().join('.') + ' ' + String(t.saat || '').slice(0, 5) })) : null));
    var not = el('p', { sinif: 'mut kucuk', metin: 'Bilgiler maskeli gösterilir.' });
    var acik = null;
    if (t.acik_yetki && s && s.acikGetir) {
      var ayKap = el('dl', { sinif: 'tk-teslim-kaydi', hidden: true });
      var gorsel = el('div', { sinif: 'tk-imza-gorsel', hidden: true });
      var ayDugme = el('button', { type: 'button', sinif: 'dugme kucuk', metin: 'Açık göster' });
      var gizle = el('button', { type: 'button', sinif: 'dugme kucuk', metin: 'Maskele', hidden: true });
      var ayIleti = el('span', { sinif: 'mut kucuk', role: 'status' });
      ayDugme.addEventListener('click', function () {
        ayDugme.disabled = true; ayIleti.textContent = 'Yükleniyor…';
        s.acikGetir(f.no).then(function (v) {
          ayIleti.textContent = 'Bu görüntüleme günlüğe yazıldı.'; ayKap.textContent = '';
          var fisTel = v.fis_telefonu === 'ayni' ? 'Fişteki numarayla aynı' : v.fis_telefonu === 'farkli' ? 'Fişteki numaradan farklı' : '';
          [['Ad soyad', v.ad], ['Tam telefon', v.tel], ['Fiş telefonu', fisTel], ['IP adresi', v.ip], ['Cihaz bilgisi (user-agent)', v.cihaz], ['Teslim eden', v.kim]].forEach(function (x) {
            if (x[1]) ayKap.appendChild(el('div', {}, el('dt', { metin: x[0] }), el('dd', { sinif: x[0] === 'Cihaz bilgisi (user-agent)' ? 'kucuk' : x[0] === 'Ad soyad' ? '' : 'mono', metin: x[1] })));
          });
          ayKap.hidden = false; ayDugme.hidden = true; gizle.hidden = false; kap._acik = anahtar;
          // Süre dolunca / yönetici oturumu kapanınca yeniden maskele.
          var maskele = function () { if (kap._acik === anahtar) { kap._acik = null; teslimKaydiCiz(kap, f, s); } };
          kap._acikSure = setTimeout(maskele, ACIK_SURE_MS);
          ACIK_GORUNUMLER.push(maskele);
          ayIleti.textContent = 'Bu görüntüleme günlüğe yazıldı. 60 sn sonra yeniden maskelenir.';
          if (t.imza_sha256 && s.imzaGetir) {
            gorsel.hidden = false; gorsel.textContent = '';
            gorsel.appendChild(el('span', { sinif: 'mut kucuk', metin: 'İmza yükleniyor…' }));
            s.imzaGetir(f.no).then(function (b) {
              if (!b) throw new Error('İmza okunamadı');
              var u = URL.createObjectURL(b);
              gorsel.textContent = '';
              var img = el('img', { src: u, alt: 'Teslim alanın imzası: ' + (v.ad || ''), sinif: 'tk-imza-resim' });
              // Bellekte tutulmaz: görüntü yüklenince adres bırakılır.
              img.addEventListener('load', function () { URL.revokeObjectURL(u); });
              gorsel.appendChild(img);
            }).catch(function (e) {
              gorsel.textContent = '';
              gorsel.appendChild(el('span', { sinif: 'dz-neden kucuk', metin: (e && e.message) || 'İmza okunamadı' }));
            });
          }
        }, function (e) {
          ayDugme.disabled = false;
          ayIleti.textContent = e && e.kod === 401 ? 'Önce yönetici girişi yapın.' : e && e.kod === 403 ? 'Bu cihazın açık görüntüleme yetkisi yok.' : hataIleti(e);
        });
      });
      gizle.addEventListener('click', function () { kap._acik = null; teslimKaydiCiz(kap, f, s); });
      acik = el('div', { sinif: 'tk-teslim-ayrinti' }, el('div', { sinif: 'dz-sira' }, ayDugme, gizle, ayIleti), ayKap, gorsel);
    }
    kap.appendChild(el('section', { sinif: 'dz tk-teslim-bolum', 'aria-label': 'Teslim kaydı' }, el('h3', { metin: 'Teslim kaydı' }), dl, not, acik));
  }
  /* İmzayı ikili olarak çeker: 404 = imza yok, 409 = dosya kayıttaki özetle uyuşmuyor. */
  function imzaCek(no, basliklar) {
    return fetch('/api/fis/' + encodeURIComponent(no) + '/imza', { headers: basliklar || {}, cache: 'no-store' }).then(function (r) {
      if (r.ok) return r.blob();
      throw new Error(r.status === 404 ? 'İmza dosyası bulunamadı' : r.status === 403 ? 'Bu cihaz teslim imzalarını göremez' : r.status === 401 ? 'İmza için yönetici girişi gerekli' : r.status === 409 ? 'İmza dosyası kayıtla uyuşmuyor ya da çözülemedi (değiştirilmiş olabilir)' : 'İmza okunamadı (' + r.status + ')');
    }, function () { throw new Error('İmza okunamadı: sunucuya ulaşılamadı'); });
  }

  window.TKD = {
    el: el, hataIleti: hataIleti, yetkiVar: yetkiVar, renkAd: renkAd, ekMetin: ekMetin,
    renkSecici: renkSecici, adetSecici: adetSecici, kalemeEkle: kalemeEkle, yerlestir: yerlestir, bagliSor: bagliSor,
    imzaAlani: imzaAlani, teslimSor: teslimSor, teslimSepeti: teslimSor, sepetAcik: sepetAcik, teslimKaydiCiz: teslimKaydiCiz, teslimMaskele: teslimMaskele, imzaCek: imzaCek, iliskiAd: iliskiAd,
    sifirla: function () { SON = null; }
  };
})();
