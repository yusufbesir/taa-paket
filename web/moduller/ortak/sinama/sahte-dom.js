// Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27]
// Testler için küçük sahte DOM (bağımlılık eklememek için). Yalnız modüllerin kullandığı yüzey vardır:
// createElement/createTextNode, appendChild/removeChild, setAttribute, textContent, value/checked,
// addEventListener/dispatchEvent/click, querySelector(All) ("etiket", "[a]", "[a=\"b\"]", "etiket[a=\"b\"]").
// Gerçek tarayıcı davranışının yerini tutmaz; tarayıcı testi ayrıca gerekir.

class Dugum {
  constructor(doc) {
    this.ownerDocument = doc;
    this.parentNode = null;
  }
  get isConnected() {
    let d = this;
    while (d.parentNode) d = d.parentNode;
    return d === this.ownerDocument.body;
  }
}

class Metin extends Dugum {
  constructor(doc, metin) {
    super(doc);
    this.nodeType = 3;
    this.textContent = String(metin);
  }
}

class Oge extends Dugum {
  constructor(doc, etiket) {
    super(doc);
    this.nodeType = 1;
    this.tagName = String(etiket).toUpperCase();
    this.childNodes = [];
    this.oznitelik = new Map();
    this.dinleyiciler = new Map();
    this.value = '';
    this.checked = false;
    this.disabled = false;
    this.hidden = false;
  }
  get children() {
    return this.childNodes.filter((c) => c.nodeType === 1);
  }
  get firstChild() {
    return this.childNodes[0] || null;
  }
  appendChild(c) {
    if (c.parentNode) c.parentNode.removeChild(c);
    c.parentNode = this;
    this.childNodes.push(c);
    return c;
  }
  removeChild(c) {
    const i = this.childNodes.indexOf(c);
    if (i < 0) throw new Error('çocuk değil');
    this.childNodes.splice(i, 1);
    c.parentNode = null;
    return c;
  }
  setAttribute(k, v) {
    this.oznitelik.set(k, String(v));
  }
  getAttribute(k) {
    return this.oznitelik.has(k) ? this.oznitelik.get(k) : null;
  }
  hasAttribute(k) {
    return this.oznitelik.has(k);
  }
  removeAttribute(k) {
    this.oznitelik.delete(k);
  }
  get textContent() {
    return this.childNodes.map((c) => c.textContent).join('');
  }
  set textContent(v) {
    for (const c of this.childNodes) c.parentNode = null;
    this.childNodes = [];
    if (v !== '') this.appendChild(new Metin(this.ownerDocument, v));
  }
  addEventListener(tur, f) {
    if (!this.dinleyiciler.has(tur)) this.dinleyiciler.set(tur, []);
    this.dinleyiciler.get(tur).push(f);
  }
  removeEventListener(tur, f) {
    const l = this.dinleyiciler.get(tur) || [];
    const i = l.indexOf(f);
    if (i >= 0) l.splice(i, 1);
  }
  /** Olayı bu öğede ve atalarında (kabarcıklanma) çalıştırır; dinleyicilerin döndürdüğü sözleri toplar. */
  dispatchEvent(olay) {
    const o = typeof olay === 'string' ? { type: olay } : olay;
    o.target = o.target || this;
    o.preventDefault = o.preventDefault || (() => {});
    const sozler = [];
    for (let d = this; d; d = d.parentNode) {
      for (const f of [...((d.dinleyiciler && d.dinleyiciler.get(o.type)) || [])]) sozler.push(f.call(d, o));
    }
    return sozler;
  }
  /** Düğmeye basar; `disabled` ise hiçbir şey olmaz. Dinleyiciler bitene kadar beklenebilir. */
  click() {
    if (this.disabled) return Promise.resolve([]);
    return Promise.all(this.dispatchEvent('click'));
  }
  /** Değer yazar ve `input` olayını tetikler. */
  yaz(deger) {
    this.value = String(deger);
    return Promise.all(this.dispatchEvent('input'));
  }
  sec(deger) {
    this.value = String(deger);
    return Promise.all(this.dispatchEvent('change'));
  }
  isaretle(v = true) {
    this.checked = v;
    return Promise.all(this.dispatchEvent('change'));
  }
  eslesir(secici) {
    const m = /^([a-zA-Z0-9]*)(?:\[([a-zA-Z0-9_-]+)(?:="([^"]*)")?\])?$/.exec(secici.trim());
    if (!m) throw new Error(`sahte DOM bu seçiciyi bilmiyor: ${secici}`);
    const [, etiket, ad, deger] = m;
    if (etiket && this.tagName !== etiket.toUpperCase()) return false;
    if (ad) {
      if (!this.oznitelik.has(ad)) return false;
      if (deger !== undefined && this.oznitelik.get(ad) !== deger) return false;
    }
    return true;
  }
  querySelectorAll(secici) {
    const bulunan = [];
    const gez = (d) => {
      for (const c of d.children) {
        if (c.eslesir(secici)) bulunan.push(c);
        gez(c);
      }
    };
    gez(this);
    return bulunan;
  }
  querySelector(secici) {
    return this.querySelectorAll(secici)[0] || null;
  }
}

/** Sahte belge kurar; `kok` body'ye bağlı boş bir div'dir. */
export function sahteBelge() {
  const dinleyiciler = new Map();
  const pencere = {
    addEventListener(tur, f) {
      if (!dinleyiciler.has(tur)) dinleyiciler.set(tur, []);
      dinleyiciler.get(tur).push(f);
    },
    removeEventListener(tur, f) {
      const l = dinleyiciler.get(tur) || [];
      const i = l.indexOf(f);
      if (i >= 0) l.splice(i, 1);
    },
    dispatchEvent(olay) {
      for (const f of [...(dinleyiciler.get(olay.type) || [])]) f(olay);
    },
    dinleyiciSayisi: (tur) => (dinleyiciler.get(tur) || []).length,
  };
  const doc = {
    defaultView: pencere,
    createElement: (e) => new Oge(doc, e),
    createTextNode: (m) => new Metin(doc, m),
  };
  doc.body = new Oge(doc, 'body');
  const kok = doc.body.appendChild(new Oge(doc, 'div'));
  return { doc, kok, pencere };
}

/** Bekleyen mikro görevleri bitirir (olay dinleyicilerindeki zincirleme sözler için). */
export function bekle() {
  return new Promise((coz) => setImmediate(coz));
}
