// GET /t/<kod> (vercel.json yeniden yazımıyla /api/t?kod=<kod>) ve /api/durum/<kod>.
import { sayfa, KOD_RE, dilSec } from "../lib/takip.mjs";
import { oku, subeOku } from "../lib/depo.mjs";

const BASLIK = {
  "content-type": "text/html; charset=utf-8",
  "cache-control": "no-store",
  "x-robots-tag": "noindex",
  "content-security-policy": "default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'self'",
};

export default {
  async fetch(istek) {
    const u = new URL(istek.url);
    const kod = u.searchParams.get("kod") || "";
    let kayit = KOD_RE.test(kod) ? await oku(kod) : null;
    if (kayit && kayit.sil <= Date.now() / 1000) kayit = null;
    if (u.searchParams.get("bicim") === "json") {
      return Response.json(kayit || { hata: "bulunamadı" }, { status: kayit ? 200 : 404, headers: { "cache-control": "no-store" } });
    }
    const dil = dilSec(u.searchParams.get("dil"), istek.headers.get("accept-language"), istek.headers.get("cookie"));
    const adres = KOD_RE.test(kod) ? `/t/${kod}` : u.pathname === "/" ? "/" : "";
    const cerez = u.searchParams.get("dil") === dil ? { "set-cookie": `dil=${dil}; Path=/; Max-Age=31536000; SameSite=Lax; Secure` } : {};
    return new Response(sayfa(kayit, { dil, adres, tablo: await subeOku(), ara: u.searchParams.get("ara") || "", sekme: u.searchParams.get("sekme") || "" }), { status: kayit ? 200 : 404, headers: { ...BASLIK, ...cerez } });
  },
};
