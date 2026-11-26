// Fişsiz teslim onayı: Vercel Blob deposu ve uç işleyicisi (yalnız Vercel; ortak işleyici ./onay.mjs).
// Bu dosya @vercel/blob'u İÇE AKTARMAZ: blob işlevleri (put, head, del, list) ve fetch dışarıdan verilir; böylece
// test/dene6.mjs sahte bir Blob ile aç → form → onay → oku → kapat → temizle akışını sınar. Gerçek Vercel
// dağıtımında DENENMEDİ (README "Vercel yolu" notu): Blob CDN önbelleği ve yeniden yazım davranışı orada ölçülmeli.
// Kayıt: onay/<t>.json. İçerik yalnız şube kodu, ürün sayısı, sunucunun genel anahtarı ve (onaylanınca) şifreli veri
// {e, iv, c}; düz metin kişisel veri yazılmaz. Blob public'tir (adres t bilinmeden tahmin edilemez; t 100 bit).
// Blob'un kendi silme zamanı yoktur: okurken süresi geçen silinir, her aç/kapat'ta onay/ öneki taranır, günlük görev
// de temizler; asıl 15 dk tavanını mağaza sunucusu sağlar (süresi geçen karekod için dakikada bir "kapat" gönderir).
// Konsola hiçbir şey yazılmaz.
import { onayIsle, ONAY_OMUR } from "./onay.mjs";
import { IMZA_BETIK_YOLU } from "./imza.mjs";

export const ONAY_ONEKI = "onay/";
const yol = (t) => `${ONAY_ONEKI}${t}.json`;
// Vercel Blob cacheControlMaxAge en az 60 sn kabul eder; okumada benzersiz ?v= ile CDN önbelleği aşılır.
const YAZ_SECENEK = { access: "public", addRandomSuffix: false, allowOverwrite: true, contentType: "application/json", cacheControlMaxAge: 60 };

/** blob: {put, head, del, list} (@vercel/blob imzalarıyla). getir: fetch. */
export function onayDeposuKur(blob, getir = (...a) => fetch(...a)) {
  const { put, head, del, list } = blob;
  async function oku(t) {
    try {
      const h = await head(yol(t));
      const r = await getir(`${h.url}?v=${Date.now()}`, { cache: "no-store" });
      return r.ok ? await r.json() : null;
    } catch {
      return null;
    }
  }
  async function yaz(kayit) {
    await put(yol(kayit.t), JSON.stringify(kayit), YAZ_SECENEK);
  }
  async function sil(t) {
    try {
      const h = await head(yol(t));
      await del(h.url);
    } catch {
      /* yoksa sorun değil */
    }
  }
  /** Süresi geçen onay kayıtlarını siler. Son yüklemesi ONAY_OMUR + 60 sn'den eski olan kesin süresi geçmiştir
   *  (kayıt yalnız sil'e kadar güncellenir, sil <= ts + 900); daha yenilerin sil alanı okunur. */
  async function temizle(simdi = Math.floor(Date.now() / 1000)) {
    let imlec;
    let silinen = 0;
    do {
      const s = await list({ prefix: ONAY_ONEKI, cursor: imlec, limit: 1000 });
      for (const b of s.blobs) {
        try {
          const yuklendi = b.uploadedAt ? Math.floor(new Date(b.uploadedAt).getTime() / 1000) : simdi;
          let gecti = yuklendi < simdi - ONAY_OMUR - 60;
          if (!gecti) {
            const r = await getir(`${b.url}?v=${Date.now()}`, { cache: "no-store" });
            const k = r.ok ? await r.json() : null;
            gecti = !k || !(k.sil > simdi);
          }
          if (gecti) {
            await del(b.url);
            silinen++;
          }
        } catch {
          /* bir sonraki turda */
        }
      }
      imlec = s.hasMore ? s.cursor : undefined;
    } while (imlec);
    return silinen;
  }
  return { oku, yaz, sil, temizle };
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
