// TAA fişsiz teslim onayı (karekodla imza): Cloudflare ve Vercel'in ORTAK kodu.
// Yusuf 2026-11-26 05:33: "Fişsiz de teslim için müşteriden imza alınsın ama imza fiziksel değil, qr kod ile olsun.
// Kimin telefonundan okunduğu yazsın loglarda, teslim tarihi ve saati yazsın. Teslim alanın ismi telefon numarası da
// yazsın. Bu bilgiler veri tabanında şifreli olarak tutulsun."
// Akış (mağaza sunucusu internete açılmaz; bulut yalnız aracıdır):
//   1) Aç:     sunucu POST /api/onay/ac (ed25519 imzalı) → bulut "onay:"+t kaydını {durum:"bekliyor"} olarak açar (≤15 dk).
//   2) Müşteri GET/POST /o/<t> (betiksiz form): ad, telefon, onay kutusu. Bulut düz metni (ad, tel, user-agent, IP,
//      zaman, dil) HEMEN sunucunun X25519 genel anahtarıyla şifreler (ECDH + HKDF-SHA256 + AES-256-GCM, aad = t);
//      düz metin hiçbir yere yazılmaz/loglanmaz, kayıtta yalnız {e, iv, c} durur.
//   3) Oku:    sunucu GET /api/onay/<t> (x-taa-zaman + x-taa-imza) → {durum} ya da {durum, e, iv, c}.
//   4) Kapat:  sunucu POST /api/onay/kapat (imzalı) → kayıt silinir.
// Bu dosya iki hedefe de aynen kopyalanır (./kopyala.sh). Konsola hiçbir şey yazılmaz: kişisel veri loglanmaz.
// Yazan: [Claude Opus 5.5, TerziTakip başlığı, 2026-11-26].
import { DILLER, SAG_SOL, METIN, dilSec } from "./diller.mjs";
import { hexBayt, kacis, dilCekmecesi, subeTablosu, CSS } from "./takip.mjs";

export const ONAY_RE = /^[a-km-np-z2-9]{20}$/;
export const ONAY_OMUR = 900; // saniye: kayıt en çok 15 dk yaşar
export const ONAY_EN_COK_POST = 5;
const ZAMAN_PAYI = 600; // aç/kapat gövdesindeki ts için saat kayması payı
const OKU_PAYI = 300;
const BILGI = new TextEncoder().encode("taa-teslim-onay-v1");
const yazi = (s) => new TextEncoder().encode(s);

const json = (v, kod = 200) =>
  new Response(JSON.stringify(v), { status: kod, headers: { "content-type": "application/json", "cache-control": "no-store" } });
const HTML_BASLIK = {
  "content-type": "text/html; charset=utf-8",
  "cache-control": "no-store",
  "x-robots-tag": "noindex",
  "referrer-policy": "no-referrer",
  "content-security-policy": "default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'",
  "x-frame-options": "DENY",
  "x-content-type-options": "nosniff",
};

async function imzaGecerli(metin, imzaHex, genelHex) {
  const genel = hexBayt(genelHex);
  const imza = hexBayt(imzaHex);
  if (!genel || genel.length !== 32) return { hata: "sunucu ayarı eksik", kod: 500 };
  if (!imza || imza.length !== 64) return { hata: "imza yok", kod: 401 };
  const anahtar = await crypto.subtle.importKey("raw", genel, { name: "Ed25519" }, false, ["verify"]);
  if (!(await crypto.subtle.verify("Ed25519", anahtar, imza, yazi(metin)))) return { hata: "imza geçersiz", kod: 401 };
  return null;
}

async function imzaliGovde(govde, imzaHex, genelHex) {
  if (typeof govde !== "string" || govde.length > 1024) return { hata: "gövde geçersiz", kod: 400 };
  const h = await imzaGecerli(govde, imzaHex, genelHex);
  if (h) return h;
  try {
    const k = JSON.parse(govde);
    return k && typeof k === "object" && !Array.isArray(k) ? { k } : { hata: "gövde geçersiz", kod: 400 };
  } catch {
    return { hata: "gövde geçersiz", kod: 400 };
  }
}

/** 1) Aç gövdesi: {"v":1,"tur":"onay_ac","t","ts","sil","s","n","pk"} (ts<sil<=ts+900). */
export async function dogrulaAc(govde, imzaHex, genelHex, simdi = Math.floor(Date.now() / 1000)) {
  const s = await imzaliGovde(govde, imzaHex, genelHex);
  if (s.hata) return s;
  const k = s.k;
  if (
    k.v !== 1 || k.tur !== "onay_ac" || typeof k.t !== "string" || !ONAY_RE.test(k.t) ||
    !Number.isInteger(k.ts) || !Number.isInteger(k.sil) ||
    typeof k.s !== "string" || !/^[A-Za-z0-9_-]{1,16}$/.test(k.s) ||
    !Number.isInteger(k.n) || k.n < 1 || k.n > 50 ||
    typeof k.pk !== "string" || !/^[0-9a-fA-F]{64}$/.test(k.pk)
  ) return { hata: "gövde biçimi geçersiz", kod: 400 };
  if (Math.abs(k.ts - simdi) > ZAMAN_PAYI) return { hata: "zaman geçersiz", kod: 400 };
  if (!(k.sil > k.ts && k.sil <= k.ts + ONAY_OMUR) || k.sil <= simdi) return { hata: "silinme zamanı geçersiz", kod: 400 };
  return { kayit: { t: k.t, s: k.s, n: k.n, pk: k.pk.toLowerCase(), sil: k.sil, durum: "bekliyor", p: 0 } };
}

/** 4) Kapat gövdesi: {"v":1,"tur":"onay_kapat","t","ts"}. */
export async function dogrulaKapat(govde, imzaHex, genelHex, simdi = Math.floor(Date.now() / 1000)) {
  const s = await imzaliGovde(govde, imzaHex, genelHex);
  if (s.hata) return s;
  const k = s.k;
  if (k.v !== 1 || k.tur !== "onay_kapat" || typeof k.t !== "string" || !ONAY_RE.test(k.t) || !Number.isInteger(k.ts)) {
    return { hata: "gövde biçimi geçersiz", kod: 400 };
  }
  if (Math.abs(k.ts - simdi) > ZAMAN_PAYI) return { hata: "zaman geçersiz", kod: 400 };
  return { t: k.t };
}

/** 3) Oku: imza "onay_oku:"+t+":"+ts metni üzerinde; |ts-şimdi| <= 300. */
export async function dogrulaOku(t, zaman, imzaHex, genelHex, simdi = Math.floor(Date.now() / 1000)) {
  if (typeof t !== "string" || !ONAY_RE.test(t)) return { hata: "bulunamadı", kod: 404 };
  if (typeof zaman !== "string" || !/^\d{1,12}$/.test(zaman)) return { hata: "zaman yok", kod: 401 };
  if (Math.abs(Number(zaman) - simdi) > OKU_PAYI) return { hata: "zaman geçersiz", kod: 401 };
  const h = await imzaGecerli(`onay_oku:${t}:${zaman}`, imzaHex, genelHex);
  return h || { tamam: true };
}

/** Ad soyad: 2..60, harf/boşluk/.-' (birleşik işaretler: Arapça hareke, Hintçe virama; Farsça/Hint yazılarındaki
 *  ZWNJ U+200C ve ZWJ U+200D dahil). Boşluklar sadeleşir. Sunucunun kuralı (taa_fis::ad_karakterleri_gecerli) bunun
 *  üst kümesidir: burada geçen her ad kasada da geçer (iki taraf aynı vektörle sınanır: test/dene6.mjs + Rust). */
export const AD_RE = /^[\p{L}\p{M} .'\-\u200C\u200D]+$/u;
export function adDogrula(x) {
  const a = String(x ?? "").normalize("NFC").replace(/’/g, "'").replace(/\s+/g, " ").trim();
  if (a.length < 2 || a.length > 60) return null;
  if (!AD_RE.test(a) || !/\p{L}/u.test(a)) return null;
  return a;
}

/** Telefon: TR varsayılan. +90 / 90 / 0 ile ya da 5'le başlayan 10 hane → +90XXXXXXXXXX; + ya da 00 ile uluslararası 8..15 hane. */
export function telNormalize(x) {
  let s = String(x ?? "").trim();
  if (s.length > 32 || !/^[+\d(][\d\s().\-\/]*$/.test(s)) return null;
  s = s.replace(/[\s().\-\/]/g, "");
  if (s.startsWith("00")) s = "+" + s.slice(2);
  if (s.startsWith("+")) {
    const r = s.slice(1);
    if (!/^\d+$/.test(r)) return null;
    if (r.startsWith("90")) return /^90[1-9]\d{9}$/.test(r) ? "+" + r : null;
    return r.length >= 8 && r.length <= 15 && r[0] !== "0" ? "+" + r : null;
  }
  if (!/^\d+$/.test(s)) return null;
  if (/^0[1-9]\d{9}$/.test(s)) return "+9" + s; // 0XXXXXXXXXX → +90XXXXXXXXXX
  if (/^90[1-9]\d{9}$/.test(s)) return "+" + s;
  if (/^[1-9]\d{9}$/.test(s)) return "+90" + s;
  return null;
}

const hex = (b) => Array.from(new Uint8Array(b), (x) => x.toString(16).padStart(2, "0")).join("");
function base64(b) {
  const u = new Uint8Array(b);
  let s = "";
  for (let i = 0; i < u.length; i += 0x8000) s += String.fromCharCode(...u.subarray(i, i + 0x8000));
  return btoa(s);
}

/** Sözleşme: geçici X25519 + ECDH(pk) → HKDF-SHA256(salt = geçici_genel || pk, info = "taa-teslim-onay-v1") → AES-256-GCM (aad = t). */
export async function sifrele(duz, pkHex, t) {
  const pk = hexBayt(pkHex);
  if (!pk || pk.length !== 32) throw new Error("anahtar");
  const gecici = await crypto.subtle.generateKey({ name: "X25519" }, true, ["deriveBits"]);
  const geciciGenel = new Uint8Array(await crypto.subtle.exportKey("raw", gecici.publicKey));
  const karsi = await crypto.subtle.importKey("raw", pk, { name: "X25519" }, false, []);
  const paylasilan = await crypto.subtle.deriveBits({ name: "X25519", public: karsi }, gecici.privateKey, 256);
  const tuz = new Uint8Array(64);
  tuz.set(geciciGenel, 0);
  tuz.set(pk, 32);
  const ikm = await crypto.subtle.importKey("raw", paylasilan, "HKDF", false, ["deriveBits"]);
  const anahtarBayt = await crypto.subtle.deriveBits({ name: "HKDF", hash: "SHA-256", salt: tuz, info: BILGI }, ikm, 256);
  const anahtar = await crypto.subtle.importKey("raw", anahtarBayt, { name: "AES-GCM" }, false, ["encrypt"]);
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const c = await crypto.subtle.encrypt({ name: "AES-GCM", iv, additionalData: yazi(t) }, anahtar, yazi(duz));
  return { e: hex(geciciGenel), iv: hex(iv), c: base64(c) };
}

// ---------------------------------------------------------------- metinler (13 dil)
export const ONAY_METIN = {
  tr: {
    b: "Teslim onayı", urun: "{n} ürün",
    aciklama: "Ürünlerinizi teslim aldığınızı onaylamak için adınızı ve telefon numaranızı yazın.",
    ad: "Ad soyad", tel: "Telefon numarası", kutu: "Ürünlerimi eksiksiz teslim aldım", gonder: "Gönder",
    gizlilik: "Bilgileriniz şifrelenir ve yalnız mağazamıza iletilir; yalnız teslim kaydı için kullanılır.",
    h_b: "Lütfen kontrol edin", h_ad: "Adınızı ve soyadınızı yazın (2–60 harf).", h_tel: "Geçerli bir telefon numarası yazın.",
    h_kutu: "Onaylamak için kutuyu işaretleyin.", h_genel: "Bir sorun oluştu. Lütfen yeniden deneyin.",
    cok: "Çok fazla deneme yapıldı. Mağaza görevlisinden yeni bir karekod isteyin.",
    doldu_b: "Karekodun süresi doldu", doldu: "Bu karekodun süresi doldu, mağaza görevlisinden yenisini isteyin.",
    alindi_b: "Onayınız alındı", alindi_m: "Teşekkür ederiz. Bu sayfayı kapatabilirsiniz.",
    tesekkur_b: "Teşekkür ederiz", tesekkur_m: "Teslim onayınız mağazamıza iletildi. Bu sayfayı kapatabilirsiniz.",
  },
  en: {
    b: "Delivery confirmation", urun: "Items: {n}",
    aciklama: "To confirm that you have received your items, enter your name and phone number.",
    ad: "Full name", tel: "Phone number", kutu: "I have received all my items in full", gonder: "Submit",
    gizlilik: "Your details are encrypted and sent only to our shop; they are used only to record the handover.",
    h_b: "Please check", h_ad: "Enter your first and last name (2–60 letters).", h_tel: "Enter a valid phone number.",
    h_kutu: "Tick the box to confirm.", h_genel: "Something went wrong. Please try again.",
    cok: "Too many attempts. Ask a shop assistant for a new QR code.",
    doldu_b: "This QR code has expired", doldu: "This QR code has expired, please ask a shop assistant for a new one.",
    alindi_b: "Your confirmation has been received", alindi_m: "Thank you. You can close this page.",
    tesekkur_b: "Thank you", tesekkur_m: "Your delivery confirmation has been sent to our shop. You can close this page.",
  },
  ar: {
    b: "تأكيد الاستلام", urun: "عدد القطع: {n}",
    aciklama: "لتأكيد استلام قطعك، اكتب اسمك ورقم هاتفك.",
    ad: "الاسم الكامل", tel: "رقم الهاتف", kutu: "استلمت جميع قطعي كاملة", gonder: "إرسال",
    gizlilik: "تُشفَّر بياناتك وتُرسل إلى متجرنا فقط، وتُستخدم لتسجيل الاستلام فقط.",
    h_b: "يرجى المراجعة", h_ad: "اكتب اسمك الأول واسم العائلة (2–60 حرفًا).", h_tel: "اكتب رقم هاتف صحيحًا.",
    h_kutu: "ضع علامة في المربع للتأكيد.", h_genel: "حدث خطأ. يرجى المحاولة مرة أخرى.",
    cok: "محاولات كثيرة جدًا. اطلب رمز QR جديدًا من موظف المتجر.",
    doldu_b: "انتهت صلاحية رمز QR", doldu: "انتهت صلاحية رمز QR هذا، اطلب رمزًا جديدًا من موظف المتجر.",
    alindi_b: "تم استلام تأكيدك", alindi_m: "شكرًا لك. يمكنك إغلاق هذه الصفحة.",
    tesekkur_b: "شكرًا لك", tesekkur_m: "تم إرسال تأكيد الاستلام إلى متجرنا. يمكنك إغلاق هذه الصفحة.",
  },
  ru: {
    b: "Подтверждение получения", urun: "Изделий: {n}",
    aciklama: "Чтобы подтвердить получение изделий, укажите имя и номер телефона.",
    ad: "Имя и фамилия", tel: "Номер телефона", kutu: "Я получил(а) все свои изделия полностью", gonder: "Отправить",
    gizlilik: "Ваши данные шифруются и передаются только нашему магазину; они используются только для учёта выдачи.",
    h_b: "Пожалуйста, проверьте", h_ad: "Укажите имя и фамилию (2–60 букв).", h_tel: "Укажите действительный номер телефона.",
    h_kutu: "Отметьте флажок для подтверждения.", h_genel: "Произошла ошибка. Попробуйте ещё раз.",
    cok: "Слишком много попыток. Попросите у сотрудника магазина новый QR-код.",
    doldu_b: "Срок действия QR-кода истёк", doldu: "Срок действия этого QR-кода истёк, попросите новый у сотрудника магазина.",
    alindi_b: "Ваше подтверждение получено", alindi_m: "Спасибо. Эту страницу можно закрыть.",
    tesekkur_b: "Спасибо", tesekkur_m: "Ваше подтверждение получения отправлено в наш магазин. Эту страницу можно закрыть.",
  },
  de: {
    b: "Übergabebestätigung", urun: "Artikel: {n}",
    aciklama: "Um zu bestätigen, dass Sie Ihre Artikel erhalten haben, geben Sie Ihren Namen und Ihre Telefonnummer ein.",
    ad: "Vor- und Nachname", tel: "Telefonnummer", kutu: "Ich habe alle meine Artikel vollständig erhalten", gonder: "Senden",
    gizlilik: "Ihre Angaben werden verschlüsselt und nur an unser Geschäft übermittelt; sie dienen nur zur Dokumentation der Übergabe.",
    h_b: "Bitte prüfen", h_ad: "Geben Sie Vor- und Nachnamen ein (2–60 Buchstaben).", h_tel: "Geben Sie eine gültige Telefonnummer ein.",
    h_kutu: "Setzen Sie das Häkchen zur Bestätigung.", h_genel: "Etwas ist schiefgelaufen. Bitte versuchen Sie es erneut.",
    cok: "Zu viele Versuche. Bitten Sie das Personal um einen neuen QR-Code.",
    doldu_b: "Dieser QR-Code ist abgelaufen", doldu: "Dieser QR-Code ist abgelaufen, bitte fragen Sie das Personal nach einem neuen.",
    alindi_b: "Ihre Bestätigung ist eingegangen", alindi_m: "Vielen Dank. Sie können diese Seite schließen.",
    tesekkur_b: "Vielen Dank", tesekkur_m: "Ihre Übergabebestätigung wurde an unser Geschäft gesendet. Sie können diese Seite schließen.",
  },
  fr: {
    b: "Confirmation de remise", urun: "Articles : {n}",
    aciklama: "Pour confirmer la réception de vos articles, indiquez votre nom et votre numéro de téléphone.",
    ad: "Nom et prénom", tel: "Numéro de téléphone", kutu: "J’ai reçu tous mes articles au complet", gonder: "Envoyer",
    gizlilik: "Vos données sont chiffrées et transmises uniquement à notre boutique ; elles servent uniquement à enregistrer la remise.",
    h_b: "Veuillez vérifier", h_ad: "Indiquez votre prénom et votre nom (2 à 60 lettres).", h_tel: "Indiquez un numéro de téléphone valide.",
    h_kutu: "Cochez la case pour confirmer.", h_genel: "Une erreur s’est produite. Veuillez réessayer.",
    cok: "Trop de tentatives. Demandez un nouveau code QR au personnel de la boutique.",
    doldu_b: "Ce code QR a expiré", doldu: "Ce code QR a expiré, demandez-en un nouveau au personnel de la boutique.",
    alindi_b: "Votre confirmation a bien été reçue", alindi_m: "Merci. Vous pouvez fermer cette page.",
    tesekkur_b: "Merci", tesekkur_m: "Votre confirmation de remise a été transmise à notre boutique. Vous pouvez fermer cette page.",
  },
  es: {
    b: "Confirmación de entrega", urun: "Artículos: {n}",
    aciklama: "Para confirmar que ha recibido sus artículos, escriba su nombre y su número de teléfono.",
    ad: "Nombre y apellidos", tel: "Número de teléfono", kutu: "He recibido todos mis artículos completos", gonder: "Enviar",
    gizlilik: "Sus datos se cifran y se envían solo a nuestra tienda; se usan solo para registrar la entrega.",
    h_b: "Revise, por favor", h_ad: "Escriba su nombre y apellidos (2–60 letras).", h_tel: "Escriba un número de teléfono válido.",
    h_kutu: "Marque la casilla para confirmar.", h_genel: "Se ha producido un error. Inténtelo de nuevo.",
    cok: "Demasiados intentos. Pida un nuevo código QR al personal de la tienda.",
    doldu_b: "Este código QR ha caducado", doldu: "Este código QR ha caducado, pida uno nuevo al personal de la tienda.",
    alindi_b: "Hemos recibido su confirmación", alindi_m: "Gracias. Puede cerrar esta página.",
    tesekkur_b: "Gracias", tesekkur_m: "Su confirmación de entrega se ha enviado a nuestra tienda. Puede cerrar esta página.",
  },
  zh: {
    b: "取件确认", urun: "件数：{n}",
    aciklama: "请填写姓名和电话号码，以确认您已取回衣物。",
    ad: "姓名", tel: "电话号码", kutu: "我已完整取回全部衣物", gonder: "提交",
    gizlilik: "您的信息经过加密，仅发送给本店，仅用于记录取件。",
    h_b: "请检查", h_ad: "请填写您的姓名（2–60 个字）。", h_tel: "请填写有效的电话号码。",
    h_kutu: "请勾选复选框以确认。", h_genel: "出现问题，请重试。",
    cok: "尝试次数过多。请向店员索取新的二维码。",
    doldu_b: "二维码已过期", doldu: "此二维码已过期，请向店员索取新的二维码。",
    alindi_b: "已收到您的确认", alindi_m: "谢谢。您可以关闭此页面。",
    tesekkur_b: "谢谢", tesekkur_m: "您的取件确认已发送至本店。您可以关闭此页面。",
  },
  hi: {
    b: "डिलीवरी की पुष्टि", urun: "सामान: {n}",
    aciklama: "अपना सामान प्राप्त करने की पुष्टि के लिए अपना नाम और फ़ोन नंबर लिखें।",
    ad: "पूरा नाम", tel: "फ़ोन नंबर", kutu: "मुझे अपना सारा सामान पूरा मिल गया है", gonder: "भेजें",
    gizlilik: "आपकी जानकारी एन्क्रिप्ट करके केवल हमारी दुकान को भेजी जाती है; इसका उपयोग केवल डिलीवरी दर्ज करने के लिए होता है।",
    h_b: "कृपया जाँचें", h_ad: "अपना पहला और अंतिम नाम लिखें (2–60 अक्षर)।", h_tel: "मान्य फ़ोन नंबर लिखें।",
    h_kutu: "पुष्टि के लिए बॉक्स पर निशान लगाएँ।", h_genel: "कुछ गलत हो गया। कृपया फिर से कोशिश करें।",
    cok: "बहुत अधिक प्रयास हो गए। दुकान के कर्मचारी से नया QR कोड माँगें।",
    doldu_b: "QR कोड की समय-सीमा समाप्त हो गई", doldu: "इस QR कोड की समय-सीमा समाप्त हो गई है, दुकान के कर्मचारी से नया माँगें।",
    alindi_b: "आपकी पुष्टि मिल गई है", alindi_m: "धन्यवाद। आप यह पेज बंद कर सकते हैं।",
    tesekkur_b: "धन्यवाद", tesekkur_m: "आपकी डिलीवरी पुष्टि हमारी दुकान को भेज दी गई है। आप यह पेज बंद कर सकते हैं।",
  },
  fa: {
    b: "تأیید تحویل", urun: "تعداد اقلام: {n}",
    aciklama: "برای تأیید دریافت اقلام خود، نام و شماره تلفن خود را بنویسید.",
    ad: "نام و نام خانوادگی", tel: "شماره تلفن", kutu: "همه اقلام خود را کامل تحویل گرفتم", gonder: "ارسال",
    gizlilik: "اطلاعات شما رمزگذاری می‌شود و فقط به فروشگاه ما فرستاده می‌شود؛ فقط برای ثبت تحویل به کار می‌رود.",
    h_b: "لطفاً بررسی کنید", h_ad: "نام و نام خانوادگی خود را بنویسید (2 تا 60 حرف).", h_tel: "یک شماره تلفن معتبر بنویسید.",
    h_kutu: "برای تأیید، کادر را علامت بزنید.", h_genel: "مشکلی پیش آمد. لطفاً دوباره تلاش کنید.",
    cok: "تعداد تلاش‌ها بیش از حد مجاز است. از کارمند فروشگاه کد QR جدید بخواهید.",
    doldu_b: "مهلت این کد QR تمام شده است", doldu: "مهلت این کد QR تمام شده است، از کارمند فروشگاه کد جدید بخواهید.",
    alindi_b: "تأیید شما دریافت شد", alindi_m: "سپاسگزاریم. می‌توانید این صفحه را ببندید.",
    tesekkur_b: "سپاسگزاریم", tesekkur_m: "تأیید تحویل شما به فروشگاه ما فرستاده شد. می‌توانید این صفحه را ببندید.",
  },
  ro: {
    b: "Confirmare de predare", urun: "Articole: {n}",
    aciklama: "Pentru a confirma că ați primit articolele, scrieți numele și numărul de telefon.",
    ad: "Nume și prenume", tel: "Număr de telefon", kutu: "Am primit toate articolele mele, complet", gonder: "Trimite",
    gizlilik: "Datele dumneavoastră sunt criptate și trimise doar magazinului nostru; sunt folosite doar pentru înregistrarea predării.",
    h_b: "Vă rugăm să verificați", h_ad: "Scrieți prenumele și numele (2–60 de litere).", h_tel: "Scrieți un număr de telefon valid.",
    h_kutu: "Bifați căsuța pentru a confirma.", h_genel: "A apărut o problemă. Vă rugăm să încercați din nou.",
    cok: "Prea multe încercări. Cereți un cod QR nou de la personalul magazinului.",
    doldu_b: "Acest cod QR a expirat", doldu: "Acest cod QR a expirat, cereți unul nou de la personalul magazinului.",
    alindi_b: "Confirmarea dumneavoastră a fost primită", alindi_m: "Vă mulțumim. Puteți închide această pagină.",
    tesekkur_b: "Vă mulțumim", tesekkur_m: "Confirmarea de predare a fost trimisă magazinului nostru. Puteți închide această pagină.",
  },
  el: {
    b: "Επιβεβαίωση παράδοσης", urun: "Είδη: {n}",
    aciklama: "Για να επιβεβαιώσετε ότι παραλάβατε τα είδη σας, γράψτε το όνομα και τον αριθμό τηλεφώνου σας.",
    ad: "Ονοματεπώνυμο", tel: "Αριθμός τηλεφώνου", kutu: "Παρέλαβα όλα τα είδη μου πλήρη", gonder: "Αποστολή",
    gizlilik: "Τα στοιχεία σας κρυπτογραφούνται και αποστέλλονται μόνο στο κατάστημά μας· χρησιμοποιούνται μόνο για την καταγραφή της παράδοσης.",
    h_b: "Παρακαλούμε ελέγξτε", h_ad: "Γράψτε όνομα και επώνυμο (2–60 γράμματα).", h_tel: "Γράψτε έναν έγκυρο αριθμό τηλεφώνου.",
    h_kutu: "Επιλέξτε το πλαίσιο για επιβεβαίωση.", h_genel: "Παρουσιάστηκε πρόβλημα. Δοκιμάστε ξανά.",
    cok: "Πάρα πολλές προσπάθειες. Ζητήστε νέο κωδικό QR από το προσωπικό του καταστήματος.",
    doldu_b: "Ο κωδικός QR έληξε", doldu: "Αυτός ο κωδικός QR έληξε, ζητήστε νέο από το προσωπικό του καταστήματος.",
    alindi_b: "Η επιβεβαίωσή σας ελήφθη", alindi_m: "Ευχαριστούμε. Μπορείτε να κλείσετε αυτή τη σελίδα.",
    tesekkur_b: "Ευχαριστούμε", tesekkur_m: "Η επιβεβαίωση παράδοσης στάλθηκε στο κατάστημά μας. Μπορείτε να κλείσετε αυτή τη σελίδα.",
  },
  bg: {
    b: "Потвърждение за получаване", urun: "Артикули: {n}",
    aciklama: "За да потвърдите, че сте получили артикулите си, въведете име и телефонен номер.",
    ad: "Име и фамилия", tel: "Телефонен номер", kutu: "Получих всички свои артикули изцяло", gonder: "Изпрати",
    gizlilik: "Данните ви се криптират и се изпращат само на нашия магазин; използват се само за отбелязване на получаването.",
    h_b: "Моля, проверете", h_ad: "Въведете име и фамилия (2–60 букви).", h_tel: "Въведете валиден телефонен номер.",
    h_kutu: "Отметнете полето, за да потвърдите.", h_genel: "Възникна проблем. Моля, опитайте отново.",
    cok: "Твърде много опити. Поискайте нов QR код от служител на магазина.",
    doldu_b: "QR кодът е изтекъл", doldu: "Този QR код е изтекъл, поискайте нов от служител на магазина.",
    alindi_b: "Потвърждението ви е получено", alindi_m: "Благодарим ви. Можете да затворите тази страница.",
    tesekkur_b: "Благодарим ви", tesekkur_m: "Потвърждението за получаване е изпратено до нашия магазин. Можете да затворите тази страница.",
  },
};
export const ONAY_ANAHTARLAR = Object.keys(ONAY_METIN.tr);

// ---------------------------------------------------------------- sayfa (betiksiz)
const SIMGE = {
  kalem: '<path d="M4 20h4L19 9l-4-4L4 16z"/><path d="M14 6l4 4"/>',
  tik: '<path d="M5 12.5l4.5 4.5L19 7.5"/>',
  saat: '<circle cx="12" cy="12" r="8"/><path d="M12 8v4.5l3 2"/>',
};
const simge = (ad) =>
  `<div class="tik" aria-hidden="true"><svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round">${SIMGE[ad]}</svg></div>`;

const ONAY_CSS = `:root{--hata:#b3261e}@media (prefers-color-scheme:dark){:root{--hata:#f2b8b5}}
.kart.onay{border-top:4px solid var(--vurgu)}.onay .rozet{margin:14px 0 0}
form.onay{display:grid;gap:16px;margin-top:18px}
form.onay .alan{display:grid;gap:6px}form.onay .alan>span{font-weight:600}
form.onay input[type=text],form.onay input[type=tel]{width:100%;min-height:48px;padding:0 14px;border:1px solid var(--cizgi);border-radius:12px;background:var(--kart);color:inherit;font:inherit;font-size:16px}
form.onay input[aria-invalid=true]{border:2px solid var(--hata)}
form.onay input:focus-visible,form.onay button:focus-visible,label.kutu:focus-within{outline:3px solid var(--vurgu);outline-offset:2px}
.alan-hata{margin:0;color:var(--hata);font-size:14px;font-weight:600}
label.kutu{display:flex;gap:12px;align-items:flex-start;padding:14px;border-radius:12px;background:var(--not);font-weight:600;cursor:pointer}
label.kutu.hatali{box-shadow:inset 0 0 0 2px var(--hata)}
label.kutu input{flex:none;width:24px;height:24px;margin:0;accent-color:var(--vurgu)}
form.onay button{min-height:52px;border:0;border-radius:12px;background:var(--vurgu);color:var(--vurgu-yazi);font:inherit;font-size:17px;font-weight:700;cursor:pointer}
.gizlilik{margin:0;color:var(--ince);font-size:13.5px}
.hatalar{margin:18px 0 0;padding:12px 14px;border-inline-start:4px solid var(--hata);border-radius:8px;background:var(--not)}
.hatalar b{display:block;color:var(--hata)}.hatalar ul{margin:6px 0 0;padding-inline-start:20px}`;

/** Teslim onayı sayfası. hal: "form" | "doldu" | "alindi" | "tesekkur" | "cok".
 *  form için: {sube, n, hatalar: {ad?, tel?, kutu?, genel?}, deger: {ad, tel}}; tablo: imzalı şube tablosu (yoksa dosyadaki). */
export function onaySayfa(hal, { dil = "tr", t = "", sube = "", n = 0, tablo = null, hatalar = {}, deger = {} } = {}) {
  if (!ONAY_METIN[dil]) dil = "tr";
  const m = ONAY_METIN[dil];
  const subeler = subeTablosu(tablo);
  const s = typeof sube === "string" && Object.hasOwn(subeler, sube) ? subeler[sube] : null;
  const marka = `<bdi lang="tr">Terzi Adem Altun${s ? ` · ${kacis(s.ad)}` : ""}</bdi>`;
  const adres = ONAY_RE.test(t) ? `/o/${t}` : "";
  const tekKart = (ikon, baslik, metin) =>
    `<section class="kart onay"><div class="hero">${simge(ikon)}<div><h2>${kacis(baslik)}</h2><p class="alt">${marka}</p></div></div><p class="aciklama">${kacis(metin)}</p></section>`;
  let govde;
  if (hal === "doldu") govde = tekKart("saat", m.doldu_b, m.doldu);
  else if (hal === "cok") govde = tekKart("saat", m.b, m.cok);
  else if (hal === "alindi") govde = tekKart("tik", m.alindi_b, m.alindi_m);
  else if (hal === "tesekkur") govde = tekKart("tik", m.tesekkur_b, m.tesekkur_m);
  else {
    const h = hatalar || {};
    const liste = ["ad", "tel", "kutu", "genel"].filter((k) => h[k]).map((k) => `<li>${kacis(m["h_" + k])}</li>`).join("");
    const ozet = liste ? `<div class="hatalar" role="alert"><b>${kacis(m.h_b)}</b><ul>${liste}</ul></div>` : "";
    const hataP = (k) => (h[k] ? `<p class="alan-hata" id="h-${k}">${kacis(m["h_" + k])}</p>` : "");
    const gecersiz = (k) => (h[k] ? ` aria-invalid="true" aria-describedby="h-${k}"` : "");
    const ad = kacis(String(deger.ad ?? "").slice(0, 60));
    const tel = kacis(String(deger.tel ?? "").slice(0, 32));
    govde = `<section class="kart onay" aria-labelledby="onay-b"><div class="hero">${simge("kalem")}
      <div><h2 id="onay-b">${kacis(m.b)}</h2><p class="alt">${marka}</p></div></div>
      <p class="rozet">${kacis(m.urun.replace("{n}", String(n)))}</p>
      <p class="aciklama">${kacis(m.aciklama)}</p>${ozet}
      <form class="onay" method="post" action="${kacis(adres)}"><input type="hidden" name="dil" value="${kacis(dil)}">
      <label class="alan"><span>${kacis(m.ad)}</span><input type="text" name="ad" value="${ad}" autocomplete="name" autocapitalize="words" spellcheck="false" minlength="2" maxlength="60" required${gecersiz("ad")}></label>${hataP("ad")}
      <label class="alan"><span>${kacis(m.tel)}</span><input type="tel" name="tel" value="${tel}" inputmode="tel" autocomplete="tel" dir="ltr" maxlength="32" placeholder="0555 123 45 67" required${gecersiz("tel")}></label>${hataP("tel")}
      <label class="kutu${h.kutu ? " hatali" : ""}"><input type="checkbox" name="onay" value="1" required${gecersiz("kutu")}><span>${kacis(m.kutu)}</span></label>${hataP("kutu")}
      <button type="submit">${kacis(m.gonder)}</button>
      <p class="gizlilik">${kacis(m.gizlilik)}</p></form></section>`;
  }
  const yon = SAG_SOL.has(dil) ? "rtl" : "ltr";
  return `<!doctype html><html lang="${dil}" dir="${yon}"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover"><meta name="robots" content="noindex,nofollow">
<meta name="referrer" content="no-referrer"><meta name="color-scheme" content="light dark"><title>${kacis(m.b)} · Terzi Adem Altun</title>
<style>${CSS}
${ONAY_CSS}</style></head><body><div class="sayfa">
<header><div class="marka"><span class="logo" aria-hidden="true">TA</span><h1>Terzi Adem Altun</h1></div>${dilCekmecesi(dil, adres)}</header>
<main>${govde}</main>
<footer><span>Terzi Adem Altun · ${kacis(m.b)}</span></footer>
</div></body></html>`;
}

// ---------------------------------------------------------------- istek işleyici
/**
 * İki hedefin ortak işleyicisi. islem: "ac" | "kapat" | "oku" | "sayfa".
 * depo: {oku(t) → kayıt|null, yaz(kayit) (kayit.sil'de kendiliğinden silinmeli), sil(t), tablo() → şube tablosu|null}.
 * Düz metin (ad, telefon, user-agent, IP) yalnız bu fonksiyonun belleğinde, şifrelemeye kadar yaşar.
 */
export async function onayIsle({ islem, t = "", istek, depo, genel, simdi = Math.floor(Date.now() / 1000) }) {
  const yontem = istek.method;
  const gecerliKayit = async (x) => {
    if (!ONAY_RE.test(x)) return null;
    const k = await depo.oku(x);
    if (k && !(k.sil > simdi)) {
      await depo.sil(x).catch(() => {});
      return null;
    }
    return k;
  };
  if (islem === "ac") {
    if (yontem !== "POST") return json({ hata: "yalnız POST" }, 405);
    const s = await dogrulaAc(await istek.text(), istek.headers.get("x-taa-imza") || "", genel, simdi);
    if (s.hata) return json({ hata: s.hata }, s.kod);
    if (await gecerliKayit(s.kayit.t)) return json({ hata: "zaten var" }, 409);
    await depo.yaz(s.kayit);
    return json({ tamam: true });
  }
  if (islem === "kapat") {
    if (yontem !== "POST") return json({ hata: "yalnız POST" }, 405);
    const s = await dogrulaKapat(await istek.text(), istek.headers.get("x-taa-imza") || "", genel, simdi);
    if (s.hata) return json({ hata: s.hata }, s.kod);
    await depo.sil(s.t);
    return json({ tamam: true });
  }
  if (islem === "oku") {
    if (yontem !== "GET") return json({ hata: "yalnız GET" }, 405);
    const s = await dogrulaOku(t, istek.headers.get("x-taa-zaman") || "", istek.headers.get("x-taa-imza") || "", genel, simdi);
    if (s.hata) return json({ hata: s.hata }, s.kod);
    const k = await gecerliKayit(t);
    if (!k) return json({ hata: "bulunamadı" }, 404);
    return k.durum === "onaylandi" ? json({ durum: "onaylandi", e: k.e, iv: k.iv, c: k.c }) : json({ durum: "bekliyor" });
  }
  // Müşteri sayfası
  if (yontem !== "GET" && yontem !== "POST") return new Response("yalnız GET/POST", { status: 405 });
  const url = new URL(istek.url);
  let form = null;
  if (yontem === "POST") {
    const g = await istek.text().catch(() => "");
    form = new URLSearchParams(g.length <= 2048 ? g : "");
  }
  const formDil = form && form.get("dil");
  const dil = formDil && DILLER.some(([d]) => d === formDil) ? formDil
    : dilSec(url.searchParams.get("dil"), istek.headers.get("accept-language"), istek.headers.get("cookie"));
  const ek = yontem === "GET" && url.searchParams.get("dil") === dil
    ? { "set-cookie": `dil=${dil}; Path=/; Max-Age=31536000; SameSite=Lax; Secure` } : {};
  const tablo = await Promise.resolve(depo.tablo ? depo.tablo() : null).catch(() => null);
  const cevap = (hal, kod, sec = {}) =>
    new Response(onaySayfa(hal, { dil, t, tablo, ...sec }), { status: kod, headers: { ...HTML_BASLIK, ...ek } });
  const k = await gecerliKayit(t);
  if (!k) return cevap("doldu", 404);
  const ortak = { sube: k.s, n: k.n };
  if (k.durum === "onaylandi") return cevap("alindi", 200, ortak);
  if (yontem === "GET") return cevap("form", 200, ortak);

  // POST: basit hız sınırı (aynı t'ye en çok 5 POST), sonra doğrulama.
  const p = (Number.isInteger(k.p) ? k.p : 0) + 1;
  if (p > ONAY_EN_COK_POST) return cevap("cok", 429, ortak);
  const adHam = form.get("ad") ?? "";
  const telHam = form.get("tel") ?? "";
  const ad = adDogrula(adHam);
  const tel = telNormalize(telHam);
  const kutu = form.get("onay") === "1";
  const hatalar = {};
  if (!ad) hatalar.ad = true;
  if (!tel) hatalar.tel = true;
  if (!kutu) hatalar.kutu = true;
  if (Object.keys(hatalar).length) {
    await depo.yaz({ ...k, p });
    return cevap("form", 400, { ...ortak, hatalar, deger: { ad: adHam, tel: telHam } });
  }
  const ip = istek.headers.get("cf-connecting-ip") || (istek.headers.get("x-forwarded-for") || "").split(",")[0].trim();
  const duz = JSON.stringify({ ad, tel, ua: (istek.headers.get("user-agent") || "").slice(0, 200), ip: ip.slice(0, 64), zaman: simdi, dil });
  let sifreli;
  try {
    sifreli = await sifrele(duz, k.pk, t);
  } catch {
    // Hata nesnesi loglanmaz/döndürülmez (düz metin sızmasın).
    await depo.yaz({ ...k, p });
    return cevap("form", 500, { ...ortak, hatalar: { genel: true }, deger: { ad: adHam, tel: telHam } });
  }
  // Kayıtta yalnız şifreli veri ve kişisel olmayan alanlar kalır (pk artık gerekmez).
  await depo.yaz({ t, s: k.s, n: k.n, sil: k.sil, durum: "onaylandi", ...sifreli });
  return cevap("tesekkur", 200, ortak);
}
