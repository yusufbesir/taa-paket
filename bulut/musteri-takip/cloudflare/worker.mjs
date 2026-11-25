// TAA müşteri takip, Cloudflare Workers + KV. Ortak kod: ./takip.mjs (bulut/musteri-takip/ortak'tan kopya).
// Ayarlar (wrangler.toml): KV bağı TAKIP; değişkenler GENEL_ANAHTAR (mağaza sunucusunun genel anahtarı, gizli değil),
// YEDEK_SAYFA (Vercel adresi). Kayıt süresi KV'nin kendi silme zamanıyla (expiration = sil) yönetilir.
import { dogrula, yeniMi, sayfa, KOD_RE } from "./takip.mjs";

const html = (h, kod = 200) =>
  new Response(h, {
    status: kod,
    headers: {
      "content-type": "text/html; charset=utf-8",
      "cache-control": "no-store",
      "x-robots-tag": "noindex",
      "content-security-policy": "default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'",
    },
  });
const json = (v, kod = 200) =>
  new Response(JSON.stringify(v), { status: kod, headers: { "content-type": "application/json", "cache-control": "no-store" } });

export default {
  async fetch(istek, env) {
    const url = new URL(istek.url);
    const yol = url.pathname;
    if (istek.method === "POST" && yol === "/api/yaz") {
      const govde = await istek.text();
      const s = await dogrula(govde, istek.headers.get("x-taa-imza") || "", env.GENEL_ANAHTAR);
      if (s.hata) return json({ hata: s.hata }, s.kod);
      const eski = await env.TAKIP.get(s.kayit.k, "json");
      if (yeniMi(eski, s.kayit)) {
        await env.TAKIP.put(s.kayit.k, JSON.stringify(s.kayit), { expiration: s.kayit.sil });
      }
      return json({ tamam: true });
    }
    const m = yol.match(/^\/(t|api\/durum)\/([^/]+)$/);
    if (istek.method === "GET" && m) {
      const kod = m[2];
      let kayit = null;
      if (KOD_RE.test(kod)) kayit = await env.TAKIP.get(kod, "json");
      if (kayit && kayit.sil <= Date.now() / 1000) kayit = null;
      if (m[1] === "api/durum") return kayit ? json(kayit) : json({ hata: "bulunamadı" }, 404);
      const yedek = env.YEDEK_SAYFA && KOD_RE.test(kod) ? `${env.YEDEK_SAYFA}/t/${kod}` : "";
      return html(sayfa(kayit, { yedek }), kayit ? 200 : 404);
    }
    if (istek.method === "GET" && yol === "/") return html(sayfa(null));
    return new Response("Bulunamadı", { status: 404 });
  },
};
