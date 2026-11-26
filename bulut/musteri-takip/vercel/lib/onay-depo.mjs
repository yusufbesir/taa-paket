// Fişsiz teslim onayı: Vercel Blob deposu ve uç işleyicisi (yalnız Vercel; ortak işleyici ./onay.mjs).
// Bu dosya @vercel/blob'u İÇE AKTARMAZ: blob işlevleri (put, head, del, list) ve fetch dışarıdan verilir; böylece
// test/dene6.mjs sahte bir Blob ile aç → form → onay → oku → kapat → temizle akışını sınar. Gerçek Vercel
// dağıtımında DENENMEDİ (README "Vercel yolu" notu): Blob CDN önbelleği ve yeniden yazım davranışı orada ölçülmeli.
// Kayıt: onay/<t>.json (şube kodu, ürün sayısı, sunucunun genel anahtarı, isteğe bağlı kalem özeti oz) ve yaz-bir-kez ek
// kayıtlar onay/<t>.sonuc.json (şifreli veri {e, iv, c}), onay/<t>.deneme-<n>.json (n = 1..5, deneme hakkı); düz metin
// kişisel veri yazılmaz. "Yalnız yoksa yaz" Blob'un put(allowOverwrite: false) koşuluyla atomiktir. Blob public'tir (adres t bilinmeden tahmin edilemez; t 100 bit).
// Blob'un kendi silme zamanı yoktur: okurken süresi geçen silinir, her aç/kapat'ta onay/ öneki taranır, günlük görev
// de temizler; asıl 15 dk tavanını mağaza sunucusu sağlar (süresi geçen karekod için dakikada bir "kapat" gönderir).
// Tarama belirteç başına en çok bir okuma yapar (yalnız ana kayıt; ekler onun sil değeriyle silinir): bir belirteçte
// 7'ye kadar blob olsa da aç/kapat yanıtı ek kayıt sayısıyla uzamaz.
// "Yok" ile "okunamadı" ayrıdır: kayıt yoksa oku null döner, ağ/depo hatasında hata atar (onay.mjs 503 verir).
// Konsola hiçbir şey yazılmaz.
import { onayIsle, ONAY_OMUR } from "./onay.mjs";
import { IMZA_BETIK_YOLU } from "./imza.mjs";

export const ONAY_ONEKI = "onay/";
const yol = (t) => `${ONAY_ONEKI}${t}.json`;
/** Ek kayıt yolu: "deneme:3" → onay/<t>.deneme-3.json (aynı önek: temizle() bunları da sil alanıyla siler). */
const ekYol = (t, ad) => `${ONAY_ONEKI}${t}.${ad.replace(/:/g, "-")}.json`;
// Vercel Blob cacheControlMaxAge en az 60 sn kabul eder; okumada benzersiz ?v= ile CDN önbelleği aşılır.
// Ana kayıt üzerine yazılabilir (kapat → "kapandi"); yaz-bir-kez kayıtlar allowOverwrite: false ile yazılır.
const YAZ_SECENEK = { access: "public", addRandomSuffix: false, allowOverwrite: true, contentType: "application/json", cacheControlMaxAge: 60 };

/** head()'in "blob yok" hatası mı? @vercel/blob BlobNotFoundError atar; api/onay.mjs sınıfın kendisini de verir
 *  (blob.bulunamadiMi). Sınıf verilmezse ad/ileti ile tanınır. */
export const varsayilanBulunamadiMi = (h) =>
  !!h && (h.name === "BlobNotFoundError" || h.constructor?.name === "BlobNotFoundError" || /BlobNotFound|does not exist/i.test(String(h.message)));

/** Blob yolundan belirteç ve ek adı: onay/<t>.json → {t, ek: ""}, onay/<t>.sonuc.json → {t, ek: "sonuc"}. */
export function yolCoz(y) {
  const m = /^onay\/([^./]+)(?:\.([^/]+))?\.json$/.exec(y || "");
  return m ? { t: m[1], ek: m[2] || "" } : null;
}

/** blob: {put, head, del, list[, bulunamadiMi]} (@vercel/blob imzalarıyla). getir: fetch. */
export function onayDeposuKur(blob, getir = (...a) => fetch(...a)) {
  const { put, head, del, list } = blob;
  const bulunamadiMi = blob.bulunamadiMi || varsayilanBulunamadiMi;
  /** Kayıt yoksa null; okunamadıysa (ağ, CDN 5xx, bozuk JSON) hata atar. */
  async function okuYol(y) {
    let h;
    try {
      h = await head(y);
    } catch (hata) {
      if (bulunamadiMi(hata)) return null;
      throw hata;
    }
    const r = await getir(`${h.url}?v=${Date.now()}`, { cache: "no-store" });
    if (r.status === 404) return null; // head ile getir arasında silinmiş
    if (!r.ok) throw new Error("blob okunamadı: HTTP " + r.status);
    return await r.json();
  }
  async function yaz(kayit) {
    await put(yol(kayit.t), JSON.stringify(kayit), YAZ_SECENEK);
  }
  /** Yalnız yoksa yazar (allowOverwrite: false; Blob varsa hata verir). Hata varlıktan değilse (ağ vb.) yeniden atılır. */
  async function yolYazYoksa(y, deger) {
    try {
      await put(y, JSON.stringify(deger), { ...YAZ_SECENEK, allowOverwrite: false });
      return true;
    } catch (hata) {
      const varMi = await head(y).then(() => true, () => false);
      if (varMi) return false;
      throw hata;
    }
  }
  async function yolSil(y) {
    try {
      const h = await head(y);
      await del(h.url);
    } catch {
      /* yoksa sorun değil */
    }
  }
  const sil = (t) => yolSil(yol(t));
  /** Süresi geçen onay kayıtlarını siler. Önce önekteki bütün bloblar listelenir ve belirtece göre gruplanır; her
   *  belirteç için YALNIZ ana kayıt (onay/<t>.json) değerlendirilir: son yüklemesi ONAY_OMUR + 60 sn'den eskiyse kesin
   *  süresi geçmiştir (kayıt yalnız sil'e kadar güncellenir, sil <= ts + 900), daha yeniyse bir kez okunup sil'ine
   *  bakılır. Ana kaydın süresi geçtiyse ekleri (sonuc, deneme-n) de silinir; ana kaydı olmayan ekler sahipsizdir
   *  (ana kayıt yalnız süresi geçince silinir) ve silinir. Ana kayıt okunamazsa o belirteç bir sonraki tura kalır. */
  async function temizle(simdi = Math.floor(Date.now() / 1000)) {
    const gruplar = new Map();
    let imlec;
    do {
      const s = await list({ prefix: ONAY_ONEKI, cursor: imlec, limit: 1000 });
      for (const b of s.blobs) {
        const c = yolCoz(b.pathname ?? new URL(b.url).pathname.slice(1));
        if (!c) continue;
        const g = gruplar.get(c.t) || { ana: null, ekler: [] };
        if (c.ek) g.ekler.push(b);
        else g.ana = b;
        gruplar.set(c.t, g);
      }
      imlec = s.hasMore ? s.cursor : undefined;
    } while (imlec);
    let silinen = 0;
    const sil1 = async (b) => {
      try {
        await del(b.url);
        silinen++;
      } catch {
        /* bir sonraki turda */
      }
    };
    for (const { ana, ekler } of gruplar.values()) {
      let gecti = true; // ana kayıt yoksa ekler sahipsiz
      if (ana) {
        try {
          const yuklendi = ana.uploadedAt ? Math.floor(new Date(ana.uploadedAt).getTime() / 1000) : simdi;
          if (yuklendi >= simdi - ONAY_OMUR - 60) {
            const r = await getir(`${ana.url}?v=${Date.now()}`, { cache: "no-store" });
            if (r.status === 404) gecti = true;
            else if (!r.ok) continue;
            else {
              const k = await r.json();
              gecti = !k || !(k.sil > simdi);
            }
          }
        } catch {
          continue; // okunamadı: bir sonraki turda
        }
        if (gecti) await sil1(ana);
      }
      if (gecti) for (const b of ekler) await sil1(b);
    }
    return silinen;
  }
  return {
    oku: (t) => okuYol(yol(t)),
    yaz,
    yazYoksa: (k) => yolYazYoksa(yol(k.t), k),
    sil,
    ekOku: (t, ad) => okuYol(ekYol(t, ad)),
    ekYazYoksa: (t, ad, deger) => yolYazYoksa(ekYol(t, ad), deger),
    ekSil: (t, ad) => yolSil(ekYol(t, ad)),
    temizle,
  };
}

/** İşlemi ve belirteci bulur. Önce asıl yoldan (/o/imza.js, /o/<t>, /api/onay/<t|ac|kapat>), sonra vercel.json yeniden yazımının
 *  sorgusundan (?islem=&t=): Vercel'in yeniden yazımda istek.url'e hangisini koyduğuna bağlı kalmamak için. */
export function onayYolu(adres) {
  const u = new URL(adres);
  const p = u.pathname;
  if (p === "/api/onay/ac" || p === "/api/onay/kapat") return { islem: p.slice(10), t: "" };
  if (p === IMZA_BETIK_YOLU) return { islem: "betik", t: "" };
  let m = p.match(/^\/o\/([^/]+)$/);
  if (m) return { islem: "sayfa", t: m[1] };
  m = p.match(/^\/api\/onay\/([^/]+)$/);
  if (m) return { islem: "oku", t: m[1] };
  return { islem: u.searchParams.get("islem") || "", t: u.searchParams.get("t") || "" };
}

const ISLEMLER = ["ac", "kapat", "oku", "sayfa", "betik"];

/** api/onay.mjs'in işleyicisi. depo: onayDeposuKur(...) + {tablo}. */
export async function vercelOnay(istek, depo, genel) {
  const { islem, t } = onayYolu(istek.url);
  if (!ISLEMLER.includes(islem)) return new Response("Bulunamadı", { status: 404 });
  const cevap = await onayIsle({ islem, t, istek, depo, genel });
  // Blob'un kendi silme zamanı olmadığı için süresi geçen onay kayıtları aç/kapat sırasında da taranır.
  if ((islem === "ac" || islem === "kapat") && cevap.status === 200) await depo.temizle().catch(() => {});
  return cevap;
}
