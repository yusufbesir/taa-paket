// POST /api/subeler: mağaza sunucusunun imzalı şube tablosu (günlük site eşitlemesi).
import { dogrulaSubeler } from "../lib/takip.mjs";
import { subeOku, subeYaz } from "../lib/depo.mjs";

export default {
  async fetch(istek) {
    const json = (v, kod = 200) => Response.json(v, { status: kod, headers: { "cache-control": "no-store" } });
    if (istek.method !== "POST") return json({ hata: "yalnız POST" }, 405);
    const s = await dogrulaSubeler(await istek.text(), istek.headers.get("x-taa-imza") || "", process.env.GENEL_ANAHTAR);
    if (s.hata) return json({ hata: s.hata }, s.kod);
    const eski = await subeOku();
    if (eski && eski.ts > s.tablo.ts) return json({ hata: "daha yeni tablo var" }, 409);
    await subeYaz(s.tablo);
    return json({ tamam: true, sube: Object.keys(s.tablo.subeler).length });
  },
};
