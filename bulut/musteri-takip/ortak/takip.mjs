// TAA müşteri takip: Cloudflare ve Vercel'in ORTAK kodu (doğrulama + sayfa).
// Mağaza sunucusu (taa-fis-sunucu, musteri_takip.rs) kaydı ed25519 ile imzalayıp POST /api/yaz'a iter.
// Kayıtta kişisel veri yoktur: {v, k: takip kodu, d: durum, s: şube kodu, t: teslim günü, ts, sil}.
// Bu dosya iki hedefe de aynen kopyalanır (tools/kopyala.sh); düzenlerken ikisini birlikte güncelleyin.
// Yazan: [Claude Opus 5.5, TerziTakip başlığı, 2026-11-26].

export const KOD_RE = /^[a-km-np-z2-9]{16}$/;
const DURUMLAR = ["alindi", "terzide", "hazir", "teslim", "iptal"];
const GUN = 86400;

function hexBayt(h) {
  if (typeof h !== "string" || !/^([0-9a-f]{2})+$/i.test(h)) return null;
  const b = new Uint8Array(h.length / 2);
  for (let i = 0; i < b.length; i++) b[i] = parseInt(h.substr(i * 2, 2), 16);
  return b;
}

/** Gövdeyi ve imzayı doğrular; geçerliyse kaydı, değilse {hata, kod} döndürür. */
export async function dogrula(govde, imzaHex, genelHex, simdi = Math.floor(Date.now() / 1000)) {
  const genel = hexBayt(genelHex);
  const imza = hexBayt(imzaHex);
  if (!genel || genel.length !== 32) return { hata: "sunucu ayarı eksik", kod: 500 };
  if (!imza || imza.length !== 64) return { hata: "imza yok", kod: 401 };
  if (typeof govde !== "string" || govde.length > 1024) return { hata: "gövde geçersiz", kod: 400 };
  const anahtar = await crypto.subtle.importKey("raw", genel, { name: "Ed25519" }, false, ["verify"]);
  const tamam = await crypto.subtle.verify("Ed25519", anahtar, imza, new TextEncoder().encode(govde));
  if (!tamam) return { hata: "imza geçersiz", kod: 401 };
  let k;
  try {
    k = JSON.parse(govde);
  } catch {
    return { hata: "gövde geçersiz", kod: 400 };
  }
  const tamSayi = (x) => Number.isInteger(x) && x > 0;
  if (
    k.v !== 1 ||
    !KOD_RE.test(k.k) ||
    !DURUMLAR.includes(k.d) ||
    typeof k.s !== "string" || k.s.length > 16 ||
    typeof k.t !== "string" || !/^(\d{4}-\d{2}-\d{2})?$/.test(k.t) ||
    !tamSayi(k.ts) || !tamSayi(k.sil)
  ) {
    return { hata: "kayıt biçimi geçersiz", kod: 400 };
  }
  if (k.ts > simdi + 600) return { hata: "kayıt zamanı ileride", kod: 400 };
  if (k.ts < simdi - 30 * GUN) return { hata: "kayıt çok eski", kod: 400 };
  if (k.sil <= simdi || k.sil > simdi + 200 * GUN) return { hata: "silinme zamanı geçersiz", kod: 400 };
  // Yalnız bilinen alanlar saklanır.
  return { kayit: { v: 1, k: k.k, d: k.d, s: k.s, t: k.t, ts: k.ts, sil: k.sil } };
}

/** Eski kayıt yenisinin üzerine yazılmasın (tekrar oynatma / sıra karışması). */
export function yeniMi(eski, yeni) {
  return !eski || yeni.ts >= eski.ts;
}

const ADIMLAR = [
  ["alindi", "Fişiniz alındı"],
  ["terzide", "Terzide"],
  ["hazir", "Hazır"],
  ["teslim", "Teslim edildi"],
];

function kacis(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
}

function gunMetni(t) {
  if (!t) return "";
  const [y, a, g] = t.split("-").map(Number);
  const aylar = ["Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim", "Kasım", "Aralık"];
  const gunler = ["Pazar", "Pazartesi", "Salı", "Çarşamba", "Perşembe", "Cuma", "Cumartesi"];
  const d = new Date(Date.UTC(y, a - 1, g));
  return `${g} ${aylar[a - 1]} ${y}, ${gunler[d.getUTCDay()]}`;
}

/** Takip sayfası (sunucuda üretilir; istemci betiği yok). */
export function sayfa(kayit, { yedek = "" } = {}) {
  let govde;
  if (!kayit) {
    govde = `<div class="kutu"><h1>Kayıt bulunamadı</h1>
      <p>Fişiniz yeni kesildiyse birkaç dakika sonra yeniden deneyin. Ürününüzü teslim aldıysanız kayıt bir süre sonra kendiliğinden silinir.</p>
      ${yedek ? `<p class="ince">Bu sayfa açılmıyorsa: <a href="${kacis(yedek)}">yedek takip sayfası</a></p>` : ""}</div>`;
  } else if (kayit.d === "iptal") {
    govde = `<div class="kutu"><h1>İşlem iptal edildi</h1><p>Bu fişteki tadilat işlemi iptal edildi. Ayrıntı için mağazamızla görüşebilirsiniz.</p></div>`;
  } else {
    const sira = ADIMLAR.findIndex(([a]) => a === kayit.d);
    const adimlar = ADIMLAR.map(([, ad], i) => {
      const s = i < sira ? "tamam" : i === sira ? "simdi" : "";
      return `<li class="${s}"><span class="nokta" aria-hidden="true"></span><span>${ad}</span></li>`;
    }).join("");
    const baslik = {
      alindi: "Ürününüz teslim alındı",
      terzide: "Ürününüz terzide",
      hazir: "Ürününüz hazır",
      teslim: "Ürününüz teslim edildi",
    }[kayit.d];
    const ek = {
      alindi: kayit.t ? `Tahmini teslim: <b>${gunMetni(kayit.t)}</b>` : "",
      terzide: kayit.t ? `Tahmini teslim: <b>${gunMetni(kayit.t)}</b>` : "",
      hazir: "Ürününüzü saat 17:00'den itibaren fişinizle birlikte teslim alabilirsiniz. 14 gün içinde alınmayan ürünlerden mesul değiliz.",
      teslim: "Bizi tercih ettiğiniz için teşekkür ederiz.",
    }[kayit.d];
    govde = `<div class="kutu"><p class="ust">Tadilat takibi${kayit.s ? ` · Şube ${kacis(kayit.s)}` : ""}</p>
      <h1>${baslik}</h1><ol class="adimlar">${adimlar}</ol>${ek ? `<p>${ek}</p>` : ""}</div>`;
  }
  return `<!doctype html><html lang="tr"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1"><meta name="robots" content="noindex,nofollow">
<meta name="referrer" content="no-referrer"><title>Tadilat takibi · Terzi Adem Altun</title>
<style>
:root{--zemin:#f6f3ee;--kart:#fff;--yazi:#1d1b18;--ince:#6b655c;--vurgu:#8a5a2b;--cizgi:#e4ddd2}
@media (prefers-color-scheme:dark){:root{--zemin:#16140f;--kart:#211e18;--yazi:#f1ece4;--ince:#a59d90;--vurgu:#d7a46b;--cizgi:#3a352c}}
*{box-sizing:border-box}body{margin:0;background:var(--zemin);color:var(--yazi);font:16px/1.5 system-ui,-apple-system,"Segoe UI",Roboto,sans-serif;min-height:100vh;display:flex;flex-direction:column;align-items:center;padding:24px 16px}
header{font-weight:700;letter-spacing:.08em;text-transform:uppercase;font-size:13px;color:var(--vurgu);margin:8px 0 20px}
.kutu{background:var(--kart);border:1px solid var(--cizgi);border-radius:16px;padding:24px;max-width:440px;width:100%}
h1{font-size:24px;margin:4px 0 18px}.ust,.ince{color:var(--ince);font-size:14px;margin:0}
.adimlar{list-style:none;padding:0;margin:0 0 18px}.adimlar li{display:flex;gap:12px;align-items:center;padding:8px 0;color:var(--ince)}
.nokta{width:14px;height:14px;border-radius:50%;border:2px solid var(--cizgi);flex:none}
.tamam{color:var(--yazi)}.tamam .nokta{background:var(--vurgu);border-color:var(--vurgu)}
.simdi{color:var(--yazi);font-weight:700}.simdi .nokta{border-color:var(--vurgu);box-shadow:0 0 0 4px color-mix(in srgb,var(--vurgu) 25%,transparent)}
a{color:var(--vurgu)}footer{color:var(--ince);font-size:12px;margin-top:20px;text-align:center}
</style></head><body><header>Terzi Adem Altun</header>${govde}
</body></html>`;
}
