// TAA müşteri takip, Cloudflare Workers + KV. Ortak kod: ./takip.mjs ve ./diller.mjs (bulut/musteri-takip/ortak'tan kopya).
// Ayarlar (wrangler.toml): KV bağı TAKIP; değişkenler GENEL_ANAHTAR (mağaza sunucusunun genel anahtarı, gizli değil),
// YEDEK_SAYFA (Vercel adresi). Teslim onayı uçları: ./onay.mjs. ?sekme=ozet|islemler|subeler|sss sayfa()'ya geçer (bilinmeyen → ozet). Kayıt süresi KV'nin kendi silme zamanıyla (expiration = sil) yönetilir.
import { dogrula, dogrulaSubeler, yeniMi, sayfa, KOD_RE, dilSec } from "./takip.mjs";
import { onayIsle } from "./onay.mjs";
import { IMZA_BETIK_YOLU } from "./imza.mjs";

// Fişsiz teslim onayı (./onay.mjs): kayıt "onay:"+t, KV silme zamanı = sil (KV en az 60 sn ileri ister; asıl süre sil ile denetlenir).
const onayDepo = (env) => ({
  oku: (t) => env.TAKIP.get("onay:" + t, "json"),
  yaz: (k) => env.TAKIP.put("onay:" + k.t, JSON.stringify(k), { expiration: Math.max(k.sil, Math.floor(Date.now() / 1000) + 60) }),
  sil: (t) => env.TAKIP.delete("onay:" + t),
  tablo: () => env.TAKIP.get("_subeler", "json"),
});

// Çekmeceden dil seçilince (?dil=xx) seçim bir yıl hatırlanır; çerezde yalnız dil kodu vardır.
const dilCerezi = (url, dil) =>
  url.searchParams.get("dil") === dil ? { "set-cookie": `dil=${dil}; Path=/; Max-Age=31536000; SameSite=Lax; Secure` } : {};
const html = (h, kod = 200, ek = {}) =>
  new Response(h, {
    status: kod,
    headers: {
      ...ek,
      "content-type": "text/html; charset=utf-8",
      "cache-control": "no-store",
      "x-robots-tag": "noindex",
      "content-security-policy": "default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'self'",
    },
  });
const json = (v, kod = 200) =>
  new Response(JSON.stringify(v), { status: kod, headers: { "content-type": "application/json", "cache-control": "no-store" } });

export default {
  async fetch(istek, env) {
    const url = new URL(istek.url);
    const yol = url.pathname;
    // Teslim onayı: POST /api/onay/ac|kapat, GET /api/onay/<t>, GET|POST /o/<t>, GET /o/imza.js (parmakla imza betiği).
    if (yol === IMZA_BETIK_YOLU) return onayIsle({ islem: "betik", istek, depo: onayDepo(env), genel: env.GENEL_ANAHTAR });
    if (yol === "/api/onay/ac" || yol === "/api/onay/kapat") {
      return onayIsle({ islem: yol.slice(10), istek, depo: onayDepo(env), genel: env.GENEL_ANAHTAR });
    }
    const om = yol.match(/^\/(o|api\/onay)\/([^/]+)$/);
    if (om) return onayIsle({ islem: om[1] === "o" ? "sayfa" : "oku", t: om[2], istek, depo: onayDepo(env), genel: env.GENEL_ANAHTAR });
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
    if (istek.method === "POST" && yol === "/api/subeler") {
      // İmzalı şube tablosu (mağaza sunucusunun günlük site eşitlemesi). Anahtar "_subeler" takip koduyla çakışmaz.
      const s = await dogrulaSubeler(await istek.text(), istek.headers.get("x-taa-imza") || "", env.GENEL_ANAHTAR);
      if (s.hata) return json({ hata: s.hata }, s.kod);
      const eski = await env.TAKIP.get("_subeler", "json");
      if (eski && eski.ts > s.tablo.ts) return json({ hata: "daha yeni tablo var" }, 409);
      await env.TAKIP.put("_subeler", JSON.stringify(s.tablo));
      return json({ tamam: true, sube: Object.keys(s.tablo.subeler).length });
    }
    const tablo = istek.method === "GET" ? await env.TAKIP.get("_subeler", "json").catch(() => null) : null;
    const ara = url.searchParams.get("ara") || "";
    const m = yol.match(/^\/(t|api\/durum)\/([^/]+)$/);
    if (istek.method === "GET" && m) {
      const kod = m[2];
      let kayit = null;
      if (KOD_RE.test(kod)) kayit = await env.TAKIP.get(kod, "json");
      if (kayit && kayit.sil <= Date.now() / 1000) kayit = null;
      if (m[1] === "api/durum") return kayit ? json(kayit) : json({ hata: "bulunamadı" }, 404);
      const yedek = env.YEDEK_SAYFA && KOD_RE.test(kod) ? `${env.YEDEK_SAYFA}/t/${kod}` : "";
      const dil = dilSec(url.searchParams.get("dil"), istek.headers.get("accept-language"), istek.headers.get("cookie"));
      const adres = KOD_RE.test(kod) ? `/t/${kod}` : "";
      return html(sayfa(kayit, { yedek, dil, adres, tablo, ara, sekme: url.searchParams.get("sekme") || "" }), kayit ? 200 : 404, dilCerezi(url, dil));
    }
    if (istek.method === "GET" && yol === "/") {
      const dil = dilSec(url.searchParams.get("dil"), istek.headers.get("accept-language"), istek.headers.get("cookie"));
      return html(sayfa(null, { dil, adres: "/", tablo, sekme: url.searchParams.get("sekme") || "" }), 200, dilCerezi(url, dil));
    }
    return new Response("Bulunamadı", { status: 404 });
  },
};
