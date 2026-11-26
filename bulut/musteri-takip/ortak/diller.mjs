// TAA müşteri takip sayfası: dil sözlüğü (Yusuf 2026-11-26 03:42: "birden fazla dil seçeneği koy,
// terzi fişini de çevirsin sadece o sayfa için"). Basılı fiş Türkçe kalır; çeviri yalnız bu sayfadadır.
// Kayıt yalnız sabit anahtarlar taşıdığı için (ürün türü, katalog işlem kodu, varyant) çeviri dış servis
// olmadan, bu sözlükle yapılır. Yeni katalog işlemi eklenince buraya da eklenmeli; eksikse Türkçe ad,
// o da yoksa "Diğer tadilat" gösterilir.
// Yazan: [Claude Opus 5.5, TerziTakip başlığı, 2026-11-26].

export const DILLER = [
  ["tr", "Türkçe"],
  ["en", "English"],
  ["ar", "العربية"],
  ["ru", "Русский"],
  ["de", "Deutsch"],
  ["fr", "Français"],
];
export const SAG_SOL = new Set(["ar"]);
// Intl tarih biçimi için yerel ayar.
export const YEREL = { tr: "tr-TR", en: "en-GB", ar: "ar", ru: "ru-RU", de: "de-DE", fr: "fr-FR" };

export const METIN = {
  tr: {
    baslik: "Tadilat takibi", sube: "Şube", dil: "Dil",
    yok_b: "Kayıt bulunamadı",
    yok_m: "Fişiniz yeni kesildiyse birkaç dakika sonra yeniden deneyin. Ürününüzü teslim aldıysanız kayıt bir süre sonra kendiliğinden silinir.",
    yedek: "Bu sayfa açılmıyorsa:", yedek_a: "yedek takip sayfası",
    iptal_b: "İşlem iptal edildi", iptal_m: "Bu fişteki tadilat işlemi iptal edildi. Ayrıntı için mağazamızla görüşebilirsiniz.",
    adim: { alindi: "Fişiniz alındı", terzide: "Terzide", hazir: "Hazır", teslim: "Teslim edildi" },
    durum: { alindi: "Ürününüz teslim alındı", terzide: "Ürününüz terzide", hazir: "Ürününüz hazır", teslim: "Ürününüz teslim edildi" },
    tahmini: "Tahmini teslim",
    hazir_m: "Ürününüzü saat 17:00'den itibaren fişinizle birlikte teslim alabilirsiniz. 14 gün içinde alınmayan ürünlerden mesul değiliz.",
    teslim_m: "Bizi tercih ettiğiniz için teşekkür ederiz.",
    fis: "Fiş", fis_tarihi: "Fiş tarihi", teslim_gunu: "Teslim", islemler: "Yapılacak işlemler", diger_islem: "Diğer tadilat",
  },
  en: {
    baslik: "Alteration tracking", sube: "Branch", dil: "Language",
    yok_b: "Record not found",
    yok_m: "If your receipt was issued just now, please try again in a few minutes. Once you have collected your item, the record is deleted automatically after a while.",
    yedek: "If this page does not open:", yedek_a: "backup tracking page",
    iptal_b: "Order cancelled", iptal_m: "The alteration on this receipt has been cancelled. Please contact our store for details.",
    adim: { alindi: "Receipt issued", terzide: "At the tailor", hazir: "Ready", teslim: "Collected" },
    durum: { alindi: "We have received your item", terzide: "Your item is at the tailor", hazir: "Your item is ready", teslim: "Your item has been collected" },
    tahmini: "Estimated ready date",
    hazir_m: "You can collect your item with your receipt from 17:00. We are not responsible for items not collected within 14 days.",
    teslim_m: "Thank you for choosing us.",
    fis: "Receipt", fis_tarihi: "Receipt date", teslim_gunu: "Ready on", islemler: "Alterations", diger_islem: "Other alteration",
  },
  ar: {
    baslik: "تتبع التعديل", sube: "الفرع", dil: "اللغة",
    yok_b: "لم يتم العثور على السجل",
    yok_m: "إذا صدر إيصالك للتو، يرجى المحاولة مرة أخرى بعد بضع دقائق. بعد استلام قطعتك يُحذف السجل تلقائيًا بعد فترة.",
    yedek: "إذا لم تفتح هذه الصفحة:", yedek_a: "صفحة التتبع الاحتياطية",
    iptal_b: "تم إلغاء الطلب", iptal_m: "تم إلغاء التعديل الخاص بهذا الإيصال. للتفاصيل يرجى التواصل مع متجرنا.",
    adim: { alindi: "تم إصدار الإيصال", terzide: "لدى الخياط", hazir: "جاهز", teslim: "تم الاستلام" },
    durum: { alindi: "استلمنا قطعتك", terzide: "قطعتك لدى الخياط", hazir: "قطعتك جاهزة", teslim: "تم استلام قطعتك" },
    tahmini: "موعد الجاهزية المتوقع",
    hazir_m: "يمكنك استلام قطعتك مع الإيصال ابتداءً من الساعة 17:00. لسنا مسؤولين عن القطع التي لا تُستلم خلال 14 يومًا.",
    teslim_m: "شكرًا لاختياركم لنا.",
    fis: "الإيصال", fis_tarihi: "تاريخ الإيصال", teslim_gunu: "موعد الاستلام", islemler: "التعديلات", diger_islem: "تعديل آخر",
  },
  ru: {
    baslik: "Отслеживание ремонта одежды", sube: "Филиал", dil: "Язык",
    yok_b: "Запись не найдена",
    yok_m: "Если чек выдан только что, попробуйте снова через несколько минут. После получения изделия запись через некоторое время удаляется автоматически.",
    yedek: "Если страница не открывается:", yedek_a: "резервная страница",
    iptal_b: "Заказ отменён", iptal_m: "Работа по этому чеку отменена. Подробности уточните в нашем магазине.",
    adim: { alindi: "Чек выдан", terzide: "У портного", hazir: "Готово", teslim: "Выдано" },
    durum: { alindi: "Мы приняли ваше изделие", terzide: "Ваше изделие у портного", hazir: "Ваше изделие готово", teslim: "Изделие выдано" },
    tahmini: "Ориентировочная дата готовности",
    hazir_m: "Изделие можно забрать с чеком начиная с 17:00. Мы не несём ответственности за изделия, не забранные в течение 14 дней.",
    teslim_m: "Спасибо, что выбрали нас.",
    fis: "Чек", fis_tarihi: "Дата чека", teslim_gunu: "Готово", islemler: "Работы", diger_islem: "Другая работа",
  },
  de: {
    baslik: "Änderungsverfolgung", sube: "Filiale", dil: "Sprache",
    yok_b: "Eintrag nicht gefunden",
    yok_m: "Wenn Ihr Beleg gerade erst ausgestellt wurde, versuchen Sie es bitte in ein paar Minuten erneut. Nach der Abholung wird der Eintrag nach einiger Zeit automatisch gelöscht.",
    yedek: "Falls diese Seite nicht lädt:", yedek_a: "Ersatzseite",
    iptal_b: "Auftrag storniert", iptal_m: "Die Änderung auf diesem Beleg wurde storniert. Für Details wenden Sie sich bitte an unser Geschäft.",
    adim: { alindi: "Beleg ausgestellt", terzide: "Beim Schneider", hazir: "Fertig", teslim: "Abgeholt" },
    durum: { alindi: "Wir haben Ihr Kleidungsstück erhalten", terzide: "Ihr Kleidungsstück ist beim Schneider", hazir: "Ihr Kleidungsstück ist fertig", teslim: "Ihr Kleidungsstück wurde abgeholt" },
    tahmini: "Voraussichtlich fertig am",
    hazir_m: "Sie können Ihr Kleidungsstück ab 17:00 Uhr mit Ihrem Beleg abholen. Für Stücke, die nicht innerhalb von 14 Tagen abgeholt werden, übernehmen wir keine Haftung.",
    teslim_m: "Vielen Dank, dass Sie sich für uns entschieden haben.",
    fis: "Beleg", fis_tarihi: "Belegdatum", teslim_gunu: "Fertig am", islemler: "Änderungen", diger_islem: "Sonstige Änderung",
  },
  fr: {
    baslik: "Suivi de retouche", sube: "Boutique", dil: "Langue",
    yok_b: "Dossier introuvable",
    yok_m: "Si votre ticket vient d'être émis, réessayez dans quelques minutes. Une fois l'article récupéré, le dossier est supprimé automatiquement après un certain temps.",
    yedek: "Si cette page ne s'ouvre pas :", yedek_a: "page de suivi de secours",
    iptal_b: "Commande annulée", iptal_m: "La retouche de ce ticket a été annulée. Pour plus de détails, contactez notre boutique.",
    adim: { alindi: "Ticket émis", terzide: "Chez le tailleur", hazir: "Prêt", teslim: "Récupéré" },
    durum: { alindi: "Nous avons reçu votre article", terzide: "Votre article est chez le tailleur", hazir: "Votre article est prêt", teslim: "Votre article a été récupéré" },
    tahmini: "Date prévue",
    hazir_m: "Vous pouvez récupérer votre article avec votre ticket à partir de 17h00. Nous déclinons toute responsabilité pour les articles non récupérés sous 14 jours.",
    teslim_m: "Merci de nous avoir choisis.",
    fis: "Ticket", fis_tarihi: "Date du ticket", teslim_gunu: "Prêt le", islemler: "Retouches", diger_islem: "Autre retouche",
  },
};

// Ürün türleri: [tr, en, ar, ru, de, fr]
const URUN = {
  pantolon: ["Pantolon", "Trousers", "بنطال", "Брюки", "Hose", "Pantalon"],
  ceket: ["Ceket", "Jacket", "سترة", "Пиджак", "Sakko", "Veste"],
  gomlek: ["Gömlek", "Shirt", "قميص", "Рубашка", "Hemd", "Chemise"],
  yelek: ["Yelek", "Waistcoat", "صديري", "Жилет", "Weste", "Gilet"],
  takim: ["Takım elbise", "Suit", "بدلة", "Костюм", "Anzug", "Costume"],
  etek: ["Etek", "Skirt", "تنورة", "Юбка", "Rock", "Jupe"],
  elbise: ["Elbise", "Dress", "فستان", "Платье", "Kleid", "Robe"],
  mont: ["Mont", "Coat", "معطف", "Куртка", "Jacke", "Blouson"],
  palto: ["Palto", "Overcoat", "معطف طويل", "Пальто", "Mantel", "Manteau"],
  bluz: ["Bluz", "Blouse", "بلوزة", "Блузка", "Bluse", "Chemisier"],
  kot: ["Kot pantolon", "Jeans", "جينز", "Джинсы", "Jeans", "Jean"],
  tisort: ["Tişört", "T-shirt", "تي شيرت", "Футболка", "T-Shirt", "T-shirt"],
  diger: ["Diğer ürün", "Other item", "قطعة أخرى", "Другое изделие", "Sonstiges", "Autre article"],
};

// Katalog işlemleri (taa-terzi islemler.json kodları): [tr, en, ar, ru, de, fr]
const ISLEM = {
  "pant-paca-kisalt": ["Paça kısaltma", "Hem shortened", "تقصير الطرف السفلي", "Укорачивание низа брюк", "Saum kürzen", "Ourlet raccourci"],
  "pant-bacak-daralt": ["Bacak daraltma", "Leg taper", "تضييق الساق", "Сужение брючин", "Bein enger machen", "Jambes resserrées"],
  "pant-bel-daralt": ["Bel daraltma", "Waist taken in", "تضييق الخصر", "Ушивание пояса", "Bund enger machen", "Taille resserrée"],
  "pant-bacak-payi-ac": ["Bacak payı açma", "Leg let out", "توسيع الساق", "Расширение брючин", "Bein auslassen", "Jambes élargies"],
  "pant-paca-payi-ac": ["Paça payı açma", "Hem let down", "تطويل الطرف السفلي", "Удлинение низа", "Saum auslassen", "Ourlet rallongé"],
  "pant-bel-payi-ac": ["Bel payı açma", "Waist let out", "توسيع الخصر", "Расширение пояса", "Bund auslassen", "Taille élargie"],
  "pant-fermuar": ["Fermuar değişimi", "Zip replacement", "تغيير السحاب", "Замена молнии", "Reißverschluss ersetzen", "Remplacement de fermeture éclair"],
  "pant-salvar-daralt": ["Şalvarlık için daraltma", "Seat taken in", "تضييق منطقة الجلوس", "Ушивание в шаге", "Gesäß enger machen", "Fond resserré"],
  "cek-etek-kisalt": ["Etek boyu kısaltma", "Jacket length shortened", "تقصير طول السترة", "Укорачивание длины", "Länge kürzen", "Longueur raccourcie"],
  "cek-kol-kisalt": ["Kol boyu kısaltma", "Sleeves shortened", "تقصير الأكمام", "Укорачивание рукавов", "Ärmel kürzen", "Manches raccourcies"],
  "cek-sirt-daralt": ["Sırt daraltma", "Back taken in", "تضييق الظهر", "Ушивание спинки", "Rücken enger machen", "Dos resserré"],
  "cek-kol-daralt": ["Kol daraltma", "Sleeves narrowed", "تضييق الأكمام", "Сужение рукавов", "Ärmel enger machen", "Manches resserrées"],
  "cek-omuz-kol-kisalt": ["Omuzdan kol kısaltma", "Sleeves shortened from the shoulder", "تقصير الأكمام من الكتف", "Укорачивание рукавов от плеча", "Ärmel an der Schulter kürzen", "Manches raccourcies par l'épaule"],
  "cek-dugme-kaydir": ["Düğme kaydırma", "Button moved", "نقل الأزرار", "Перешив пуговиц", "Knopf versetzen", "Bouton déplacé"],
  "cek-dugme-saglam": ["Düğme sağlamlaştırma", "Buttons secured", "تثبيت الأزرار", "Укрепление пуговиц", "Knöpfe befestigen", "Boutons consolidés"],
  "cek-vatka-kisalt": ["Omuz vatkası kısaltma", "Shoulder pads reduced", "تقليل حشوة الكتف", "Уменьшение плечевых накладок", "Schulterpolster verkleinern", "Épaulettes réduites"],
  "cek-yirtmac-kapat": ["Yırtmaç kapama", "Vent closed", "إغلاق الشق", "Зашивание шлицы", "Schlitz schließen", "Fente fermée"],
  "gom-kol-kisalt": ["Kol kısaltma", "Sleeves shortened", "تقصير الأكمام", "Укорачивание рукавов", "Ärmel kürzen", "Manches raccourcies"],
  "gom-daralt": ["Daraltma", "Taken in", "تضييق", "Ушивание", "Enger machen", "Resserrage"],
  "gom-dugme-kaydir": ["Düğme kaydırma", "Button moved", "نقل الأزرار", "Перешив пуговиц", "Knopf versetzen", "Bouton déplacé"],
  "gom-yaka-sabitle": ["Yaka sabitleme", "Collar secured", "تثبيت الياقة", "Закрепление воротника", "Kragen fixieren", "Col fixé"],
  "yel-daralt": ["Daraltma", "Taken in", "تضييق", "Ушивание", "Enger machen", "Resserrage"],
  "yel-omuz-kisalt": ["Omuzdan kısaltma", "Shortened at the shoulder", "تقصير من الكتف", "Укорачивание по плечу", "An der Schulter kürzen", "Raccourci par l'épaule"],
  "yel-kulak-kapat": ["Koltuk altı kulak kapama", "Armhole closed", "تضييق فتحة الإبط", "Ушивание проймы", "Armloch verkleinern", "Emmanchure resserrée"],
  "takim-beden-daralt": ["Komple bir beden daraltma", "Taken in one size", "تصغير مقاس واحد", "Ушивание на один размер", "Um eine Größe enger", "Resserrage d'une taille"],
  "pant-beden-daralt": ["Komple bir beden daraltma", "Taken in one size", "تصغير مقاس واحد", "Ушивание на один размер", "Um eine Größe enger", "Resserrage d'une taille"],
  "cek-beden-daralt": ["Bir beden daraltma", "Taken in one size", "تصغير مقاس واحد", "Ушивание на один размер", "Um eine Größe enger", "Resserrage d'une taille"],
  "yel-beden-daralt": ["Bir beden daraltma", "Taken in one size", "تصغير مقاس واحد", "Ушивание на один размер", "Um eine Größe enger", "Resserrage d'une taille"],
  "yel-genislet-parca": ["Genişletme (parça ekleyerek)", "Let out (with added panel)", "توسيع (بإضافة قطعة)", "Расширение (вставкой)", "Erweitern (mit Einsatz)", "Élargissement (par ajout de pièce)"],
  "gom-etek-kisalt": ["Etek kısaltma", "Hem shortened", "تقصير الطول", "Укорачивание низа", "Saum kürzen", "Bas raccourci"],
  "takim-3parca-beden-daralt": ["Takım elbise (ceket+pantolon+yelek) bir beden daraltma", "Three-piece suit taken in one size", "تصغير بدلة من ثلاث قطع مقاسًا واحدًا", "Ушивание костюма-тройки на один размер", "Dreiteiliger Anzug um eine Größe enger", "Costume trois pièces resserré d'une taille"],
};

const VARYANT = {
  duz: ["Düz paça", "Plain hem", "طرف عادي", "Без отворота", "Ohne Aufschlag", "Ourlet simple"],
  duble: ["Duble paça", "Turn-up hem", "طرف مزدوج", "С отворотом", "Mit Aufschlag", "Ourlet à revers"],
};

const SIRA = DILLER.map(([d]) => d);
const al = (tablo, anahtar, dil) => {
  const s = tablo[anahtar];
  return s ? s[SIRA.indexOf(dil)] || s[0] : null;
};
export const urunAdi = (u, dil) => al(URUN, u, dil) || al(URUN, "diger", dil);
export const islemAdi = (i, dil) => (i ? al(ISLEM, i, dil) : null);
export const varyantAdi = (v, dil) => (v ? al(VARYANT, v, dil) : null);

/** Dil seçimi: önce ?dil=, sonra tarayıcının Accept-Language başlığı, yoksa Türkçe. */
export function dilSec(sorgu, kabul) {
  if (sorgu && METIN[sorgu]) return sorgu;
  for (const parca of String(kabul || "").split(",")) {
    const d = parca.trim().slice(0, 2).toLowerCase();
    if (METIN[d]) return d;
  }
  return "tr";
}
