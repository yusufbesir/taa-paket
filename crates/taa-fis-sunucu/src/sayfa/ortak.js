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
     Sunucu ucu: POST /api/fis/<no>/teslim-al {fisli, teslim_alan, iliski, telefon_son4?, imza_png_base64?, bagli?}.
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

  /* Teslim iletişim kutusu. s: {fis, oturum, api(metot, yol, govde)}.
     Sonuç: sunucunun 200 yanıtı ({tamam, birlikte_teslim, teslim_kaydi}) ya da vazgeçilirse null.
     409 bagli_teslim gelirse bağlı fiş kutusu (bagliSor) bu kutunun üstünde açılır; seçimden sonra aynı istek
     (son 4 ve imza dahil) "bagli" listesiyle yeniden gönderilir. Asıl yetki denetimi sunucudadır (403). */
  function teslimSor(s) {
    var f = s.fis || {}, no = f.no, fissizYetki = yetkiVar(s.oturum, 'fissiz-teslim');
    return new Promise(function (bitti) {
      var bas = kimlik('tsk'), onceki = document.activeElement, sonuc = null, bitmis = false, mesgul = false;
      var gFisli = el('input', { type: 'radio', name: bas + '-tur', value: 'fisli' });
      var gFissiz = el('input', { type: 'radio', name: bas + '-tur', value: 'fissiz', disabled: !fissizYetki });
      gFisli.checked = true;
      var fissizNeden = fissizYetki ? null : el('span', { sinif: 'dz-neden kucuk', id: bas + '-yn',
        metin: 'Bu cihazda fişsiz teslim yetkisi yok. Mağaza bilgisayarından ya da yöneticinin Telefonlar ekranında yetki verdiği bir cihazdan yapılır.' });
      if (fissizNeden) gFissiz.setAttribute('aria-describedby', bas + '-yn');
      var ad = el('input', { id: bas + '-ad', maxlength: '80', autocomplete: 'off', spellcheck: 'false' });
      ad.value = f.musteri || '';
      var iliski = el('select', { id: bas + '-il' }, ILISKI.map(function (x) { return el('option', { value: x[0], metin: x[1] }); }));
      iliski.value = 'kendisi';
      var son4 = el('input', { id: bas + '-s4', inputmode: 'numeric', autocomplete: 'off', maxlength: '4', pattern: '[0-9]{4}', sinif: 'mono',
        'aria-describedby': bas + '-s4a' });
      // Yeni haneler yazılırken önceki "Kalan deneme" iletisi kalkar.
      son4.addEventListener('input', function () { var t = son4.value.replace(/\D/g, '').slice(0, 4); if (t !== son4.value) son4.value = t; mesaj.hidden = true; });
      var alanSon4 = el('div', { sinif: 'alan tk-son4' }, el('label', { for: bas + '-s4', metin: 'Müşteri telefonunun son 4 hanesi' }), son4,
        el('span', { sinif: 'mut kucuk', id: bas + '-s4a', metin: 'Teslim alana sorun; ekranda gösterilmez, kaydedilmez.' }));
      var mesaj = el('div', { sinif: 'mesaj hata kucuk', role: 'alert', hidden: true });
      var imzaAl = el('input', { type: 'checkbox' });
      var imzaAlSatir = el('label', { sinif: 'onay' }, imzaAl, el('span', { metin: 'İmza al (isteğe bağlı)' }));
      var imza = imzaAlani({ degisti: function () { mesaj.hidden = true; } });
      var imzaBas = el('div', { sinif: 'tk-imza-bas', metin: 'Teslim alanın imzası' });
      var imzaKutu = el('div', { sinif: 'tk-imza-kap' }, imzaBas, imza.kok);
      var gonderD = el('button', { type: 'submit', sinif: 'dugme sonraki', metin: 'Teslim et' });
      var vazgec = el('button', { type: 'button', sinif: 'dugme', metin: 'Vazgeç' });
      var form = el('form', { method: 'dialog', novalidate: true },
        el('h2', { id: bas, metin: 'Teslim et · ' + (no || '') }),
        el('p', { sinif: 'mut', metin: (f.musteri ? f.musteri + ' · ' : '') + 'Ürünü kimin teslim aldığını kaydedin.' }),
        el('fieldset', { sinif: 'tk-teslim-tur' }, el('legend', { metin: 'Müşteri fişini getirdi mi?' }),
          el('label', { sinif: 'onay' }, gFisli, el('span', { metin: 'Fişiyle geldi' })),
          el('label', { sinif: 'onay' + (fissizYetki ? '' : ' dz-pasif') }, gFissiz, el('span', { metin: 'Fişi yok' })),
          fissizNeden),
        el('div', { sinif: 'dz-izgara' },
          el('div', { sinif: 'alan' }, el('label', { for: bas + '-ad', metin: 'Teslim alan (ad soyad)' }), ad),
          el('div', { sinif: 'alan' }, el('label', { for: bas + '-il', metin: 'Müşteriye yakınlığı' }), iliski)),
        alanSon4, imzaAlSatir, imzaKutu, mesaj,
        el('div', { sinif: 'dz-sira' }, vazgec, gonderD));
      var kutu = el('dialog', { sinif: 'dz-kutu tk-teslim', 'aria-labelledby': bas }, form);
      function fissiz() { return gFissiz.checked && fissizYetki; }
      function goster() {
        var fz = fissiz();
        alanSon4.hidden = !fz;
        imzaAlSatir.hidden = fz;
        imzaKutu.hidden = !(fz || imzaAl.checked);
        imzaBas.textContent = fz ? 'Teslim alanın imzası (zorunlu)' : 'Teslim alanın imzası';
        if (!imzaKutu.hidden) imza.boyutla();
      }
      [gFisli, gFissiz, imzaAl].forEach(function (x) { x.addEventListener('change', function () { mesaj.hidden = true; goster(); }); });
      function hata(t, odak) { mesaj.textContent = t; mesaj.hidden = false; if (odak) try { odak.focus(); } catch (e) {} }
      function kapat(r) { sonuc = r; if (kutu.open && kutu.close) kutu.close(); else son(); }
      function son() {
        if (bitmis) return; bitmis = true;
        if (kutu.parentNode) kutu.parentNode.removeChild(kutu);
        if (onceki && onceki.focus) try { onceki.focus(); } catch (e) {}
        bitti(sonuc);
      }
      function mesgulYap(m) { mesgul = m; gonderD.disabled = m; vazgec.disabled = m; gonderD.textContent = m ? 'Gönderiliyor…' : 'Teslim et'; }
      function gonder(govde) {
        mesgulYap(true);
        return s.api('POST', '/api/fis/' + encodeURIComponent(no) + '/teslim-al', govde).then(function (r) {
          mesgulYap(false); kapat(r || { tamam: true });
        }, function (e) {
          mesgulYap(false);
          var v = (e && e.veri) || {};
          if (e && e.kod === 409 && v.soru === 'bagli_teslim') {
            return bagliSor(v, {}).then(function (sec) {
              if (!sec) { hata('Teslim edilmedi: bağlı fiş sorusundan vazgeçildi.'); return; }
              var g2 = {}; Object.keys(govde).forEach(function (k) { g2[k] = govde[k]; }); g2.bagli = sec;
              return gonder(g2);
            });
          }
          if (v.hata === 'son4_yanlis') {
            son4.value = '';
            hata(v.kilitli || v.kalan_hak === 0 ? 'Son 4 hane yanlış. Bu fiş 15 dakika kilitlendi; fişsiz teslim şimdilik yapılamaz.'
              : 'Son 4 hane yanlış. Kalan deneme: ' + v.kalan_hak + '.', son4);
            return;
          }
          if (v.hata === 'son4_kilitli') {
            var dk = Math.max(1, Math.ceil((+v.kalan_sn || 0) / 60));
            hata('Çok fazla yanlış deneme. Fişsiz teslim bu fiş için yaklaşık ' + dk + ' dakika kilitli.');
            return;
          }
          if (e && e.kod === 413) { hata('İmza çok büyük. Temizleyip daha sade imzalatın.'); return; }
          hata(hataIleti(e));
        });
      }
      form.addEventListener('submit', function (ev) {
        ev.preventDefault();
        if (mesgul) return;
        mesaj.hidden = true;
        var fz = fissiz(), a = ad.value.trim().replace(/\s+/g, ' ');
        if (a.split(' ').length < 2) { hata('Teslim alanın adını ve soyadını yazın.', ad); return; }
        if (a.length > 80) { hata('Ad soyad en çok 80 harf olabilir.', ad); return; }
        if (!AD_DESENI.test(a)) { hata('Ad soyadda yalnız harf, boşluk, tire, kesme işareti ve nokta olabilir.', ad); return; }
        var govde = { fisli: !fz, teslim_alan: a, iliski: iliski.value };
        if (fz) {
          if (!/^\d{4}$/.test(son4.value)) { hata('Müşteri telefonunun son 4 hanesini yazın.', son4); return; }
          govde.telefon_son4 = son4.value;
        }
        if (fz || imzaAl.checked) {
          if (imza.bos()) { hata(fz ? 'Fişsiz teslimde imza zorunlu. Teslim alana dokunmatik ekranda ya da fareyle imzalatın.' : 'İmza alanı boş. İmzalatın ya da "İmza al" işaretini kaldırın.', imza.temizleDugmesi); return; }
          var p = imza.png();
          if (p.length * 3 / 4 > IMZA_SINIR) { hata('İmza çok büyük. Temizleyip daha sade imzalatın.'); return; }
          govde.imza_png_base64 = p;
        }
        gonder(govde);
      });
      vazgec.addEventListener('click', function () { kapat(null); });
      kutu.addEventListener('close', son);
      kutu.addEventListener('cancel', function (e) { if (mesgul) { e.preventDefault(); return; } sonuc = null; });
      document.body.appendChild(kutu);
      if (kutu.showModal) kutu.showModal(); else kutu.setAttribute('open', '');
      goster();
      gFisli.focus();
    });
  }

  /* Fiş ayrıntısında teslim kaydı. s.imzaGetir(no) -> Promise<Blob>: her sayfa kendi kimlik başlığıyla çeker.
     s.imzaGorebilir: false ise (usta/kişisel telefon) imza görüntüsü istenmez, yalnız "İmzalı" yazar; sunucu da 403 döner.
     İmza görüntüsü fiş + özet başına bir kez çekilir (20 sn'lik yenilemede yeniden indirilmez). */
  var IMZA_URL = {};
  function teslimKaydiCiz(kap, f, s) {
    if (!kap) return;
    kap.textContent = '';
    var t = f && f.teslim_kaydi;
    if (!t) return;
    var dl = el('dl', { sinif: 'tk-teslim-kaydi' },
      el('div', {}, el('dt', { metin: 'Teslim alan' }), el('dd', { metin: t.teslim_alan || '' })),
      el('div', {}, el('dt', { metin: 'İlişki' }), el('dd', { metin: iliskiAd(t.iliski) })),
      el('div', {}, el('dt', { metin: 'Fiş' }), el('dd', { metin: t.fisli ? 'Fişli' : 'Fişsiz (telefon son 4 ile doğrulandı)' })),
      el('div', {}, el('dt', { metin: 'İmza' }), el('dd', { metin: t.imza_sha256 ? 'İmzalı' : 'İmzasız' })),
      (t.ana_fis ? el('div', {}, el('dt', { metin: 'Birlikte teslim' }), el('dd', { sinif: 'mono', metin: t.ana_fis })) : null),
      (t.tarih ? el('div', {}, el('dt', { metin: 'Zaman' }), el('dd', { sinif: 'mono', metin: String(t.tarih).split('-').reverse().join('.') + ' ' + String(t.saat || '').slice(0, 5) })) : null));
    var gorsel = el('div', { sinif: 'tk-imza-gorsel' });
    var gorebilir = !s || s.imzaGorebilir !== false;
    kap.appendChild(el('section', { sinif: 'dz tk-teslim-bolum', 'aria-label': 'Teslim kaydı' }, el('h3', { metin: 'Teslim kaydı' }), dl, t.imza_sha256 && gorebilir ? gorsel : null));
    if (!t.imza_sha256 || !gorebilir || !s || !s.imzaGetir) return;
    var anahtar = f.no + ':' + t.imza_sha256;
    function koy(u) { gorsel.textContent = ''; gorsel.appendChild(el('img', { src: u, alt: 'Teslim alanın imzası: ' + (t.teslim_alan || ''), sinif: 'tk-imza-resim' })); }
    if (IMZA_URL[anahtar]) { koy(IMZA_URL[anahtar]); return; }
    gorsel.appendChild(el('span', { sinif: 'mut kucuk', metin: 'İmza yükleniyor…' }));
    s.imzaGetir(f.no).then(function (b) {
      if (!b) throw new Error('İmza okunamadı');
      // Bellekte en çok 20 imza tutulur.
      var k = Object.keys(IMZA_URL);
      if (k.length >= 20) { URL.revokeObjectURL(IMZA_URL[k[0]]); delete IMZA_URL[k[0]]; }
      IMZA_URL[anahtar] = URL.createObjectURL(b);
      koy(IMZA_URL[anahtar]);
    }).catch(function (e) {
      gorsel.textContent = '';
      gorsel.appendChild(el('span', { sinif: 'dz-neden kucuk', metin: (e && e.message) || 'İmza okunamadı' }));
    });
  }
  /* İmzayı ikili olarak çeker: 404 = imza yok, 409 = dosya kayıttaki özetle uyuşmuyor. */
  function imzaCek(no, basliklar) {
    return fetch('/api/fis/' + encodeURIComponent(no) + '/imza', { headers: basliklar || {}, cache: 'no-store' }).then(function (r) {
      if (r.ok) return r.blob();
      throw new Error(r.status === 404 ? 'İmza dosyası bulunamadı' : r.status === 403 ? 'Bu cihaz teslim imzalarını göremez' : r.status === 409 ? 'İmza dosyası kayıtla uyuşmuyor (değiştirilmiş olabilir)' : 'İmza okunamadı (' + r.status + ')');
    }, function () { throw new Error('İmza okunamadı: sunucuya ulaşılamadı'); });
  }

  window.TKD = {
    el: el, hataIleti: hataIleti, yetkiVar: yetkiVar, renkAd: renkAd, ekMetin: ekMetin,
    renkSecici: renkSecici, adetSecici: adetSecici, kalemeEkle: kalemeEkle, yerlestir: yerlestir, bagliSor: bagliSor,
    imzaAlani: imzaAlani, teslimSor: teslimSor, teslimKaydiCiz: teslimKaydiCiz, imzaCek: imzaCek, iliskiAd: iliskiAd,
    sifirla: function () { SON = null; }
  };
})();
