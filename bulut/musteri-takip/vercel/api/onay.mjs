// Fişsiz teslim onayı (ortak işleyici ../lib/onay.mjs, Blob deposu ../lib/onay-depo.mjs). vercel.json yeniden yazımları:
//   POST /api/onay/ac → ?islem=ac, POST /api/onay/kapat → ?islem=kapat, GET /api/onay/<t> → ?islem=oku&t=<t>,
//   GET|POST /o/<t> → ?islem=sayfa&t=<t>. İşlem asıl yoldan da bulunur (onayYolu). Kişisel veri loglanmaz.
import { put, head, del, list } from "@vercel/blob";
import { onayDeposuKur, vercelOnay } from "../lib/onay-depo.mjs";
import { subeOku } from "../lib/depo.mjs";

const depo = { ...onayDeposuKur({ put, head, del, list }), tablo: subeOku };

export default {
  fetch: (istek) => vercelOnay(istek, depo, process.env.GENEL_ANAHTAR),
};
