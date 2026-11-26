// Vercel Blob deposu: takip/<kod>.json. BLOB_READ_WRITE_TOKEN Vercel'de depo bağlanınca kendiliğinden gelir.
import { put, head, del, list } from "@vercel/blob";

const yol = (kod) => `takip/${kod}.json`;

export async function oku(kod) {
  try {
    const h = await head(yol(kod));
    const r = await fetch(`${h.url}?v=${Date.now()}`, { cache: "no-store" });
    return r.ok ? await r.json() : null;
  } catch {
    return null;
  }
}

export async function yaz(kayit) {
  await put(yol(kayit.k), JSON.stringify(kayit), {
    access: "public",
    addRandomSuffix: false,
    allowOverwrite: true,
    contentType: "application/json",
    cacheControlMaxAge: 60,
  });
}

/** Silinme zamanı geçenleri siler (takip/ ve teslim onayı onay/ önekleri; önek verilirse yalnız o). */
export async function temizle(simdi = Math.floor(Date.now() / 1000), onekler = ["takip/", "onay/"]) {
  let silinen = 0;
  for (const onek of onekler) silinen += await onekTemizle(onek, simdi);
  return silinen;
}

async function onekTemizle(onek, simdi) {
  let imlec;
  let silinen = 0;
  do {
    const s = await list({ prefix: onek, cursor: imlec, limit: 1000 });
    for (const b of s.blobs) {
      try {
        const r = await fetch(b.url, { cache: "no-store" });
        const k = r.ok ? await r.json() : null;
        if (!k || !(k.sil > simdi)) {
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

/** İmzalı şube tablosu: ayar/subeler.json (kamuya açık işletme bilgisi). */
export async function subeOku() {
  try {
    const h = await head("ayar/subeler.json");
    const r = await fetch(`${h.url}?v=${Date.now()}`, { cache: "no-store" });
    return r.ok ? await r.json() : null;
  } catch {
    return null;
  }
}

export async function subeYaz(tablo) {
  await put("ayar/subeler.json", JSON.stringify(tablo), {
    access: "public", addRandomSuffix: false, allowOverwrite: true, contentType: "application/json", cacheControlMaxAge: 60,
  });
}

// Fişsiz teslim onayı deposu ayrı dosyadadır (./onay-depo.mjs; @vercel/blob dışarıdan verilir, testte sahte Blob).
// temizle() günlük görevde onay/ önekini de tarar.
