// Fişsiz teslim onayı (ortak işleyici ../lib/onay.mjs, Blob deposu ../lib/onay-depo.mjs). vercel.json yeniden yazımları:
//   POST /api/onay/ac → ?islem=ac, POST /api/onay/kapat → ?islem=kapat, GET /api/onay/<t> → ?islem=oku&t=<t>,
//   GET /o/imza.js → ?islem=betik (parmakla imza betiği; /o/:t'den ÖNCE), GET|POST /o/<t> → ?islem=sayfa&t=<t>. İşlem asıl yoldan da bulunur (onayYolu). Kişisel veri loglanmaz.
// Ana kayıt onay/<t>.json; yaz-bir-kez ek kayıtlar onay/<t>.sonuc.json ve onay/<t>.deneme-<n>.json (koşullu put, onay-depo.mjs).
import { put, head, del, list, BlobNotFoundError } from "@vercel/blob";
import { onayDeposuKur, vercelOnay } from "../lib/onay-depo.mjs";
import { subeOku } from "../lib/depo.mjs";

// bulunamadiMi: head()'in "blob yok" hatasını ağ/depo hatasından ayırır (kapat okunamayan kaydı "yok" saymasın).
const bulunamadiMi = (h) => h instanceof BlobNotFoundError;
const depo = { ...onayDeposuKur({ put, head, del, list, bulunamadiMi }), tablo: subeOku };

export default {
  fetch: (istek) => vercelOnay(istek, depo, process.env.GENEL_ANAHTAR),
};
