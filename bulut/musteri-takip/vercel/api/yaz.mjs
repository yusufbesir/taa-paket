import { dogrula, yeniMi } from "../lib/takip.mjs";
import { oku, yaz } from "../lib/depo.mjs";

export default {
  async fetch(istek) {
    const json = (v, kod = 200) => Response.json(v, { status: kod, headers: { "cache-control": "no-store" } });
    if (istek.method !== "POST") return json({ hata: "yalnız POST" }, 405);
    const govde = await istek.text();
    const s = await dogrula(govde, istek.headers.get("x-taa-imza") || "", process.env.GENEL_ANAHTAR);
    if (s.hata) return json({ hata: s.hata }, s.kod);
    if (yeniMi(await oku(s.kayit.k), s.kayit)) await yaz(s.kayit);
    return json({ tamam: true });
  },
};
