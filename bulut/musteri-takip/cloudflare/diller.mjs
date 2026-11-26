// TAA müşteri takip sayfası: dil sözlüğü.
// Yusuf 2026-11-26 03:42: "birden fazla dil seçeneği koy, terzi fişini de çevirsin sadece o sayfa için".
// Yusuf 2026-11-26 03:53: İspanyolca, Mandarin, Hintçe, Farsça, Romence, Yunanca, Bulgarca da; dil telefonun
// sistem dilinden otomatik seçilsin; sağ üstte kısaltmalı (ör. FR) çekmeceli dil seçici.
// Basılı fiş Türkçe kalır; çeviri yalnız bu sayfadadır. Kayıt yalnız sabit anahtarlar taşıdığı için
// (ürün türü, katalog işlem kodu, varyant) çeviri dış servis olmadan, bu sözlükle yapılır. Yeni katalog
// işlemi eklenince buraya da eklenmeli; eksikse Türkçe ad, o da yoksa "Diğer tadilat" gösterilir.
// Yazan: [Claude Opus 5.5, TerziTakip başlığı, 2026-11-26].

// [kod, kendi dilindeki adı, kısaltma]
export const DILLER = [
  ["tr", "Türkçe", "TR"],
  ["en", "English", "EN"],
  ["ar", "العربية", "AR"],
  ["ru", "Русский", "RU"],
  ["de", "Deutsch", "DE"],
  ["fr", "Français", "FR"],
  ["es", "Español", "ES"],
  ["zh", "中文（普通话）", "ZH"],
  ["hi", "हिन्दी", "HI"],
  ["fa", "فارسی", "FA"],
  ["ro", "Română", "RO"],
  ["el", "Ελληνικά", "EL"],
  ["bg", "Български", "BG"],
];
export const SAG_SOL = new Set(["ar", "fa"]);
// Intl tarih biçimi için yerel ayar (Farsçada Gregoryen takvim zorlanır: fiş günü karışmasın).
export const YEREL = {
  tr: "tr-TR", en: "en-GB", ar: "ar-EG-u-nu-latn", ru: "ru-RU", de: "de-DE", fr: "fr-FR", es: "es-ES",
  zh: "zh-CN", hi: "hi-IN", fa: "fa-IR-u-ca-gregory-nu-latn", ro: "ro-RO", el: "el-GR", bg: "bg-BG",
};

export const METIN = {
  tr: {
    guncel: "Güncel", ozet: "Fiş özeti",
    baslik: "Tadilat takibi", sube: "Şube", dil: "Dil",
    yok_b: "Kayıt bulunamadı",
    yok_m: "Fişiniz yeni kesildiyse birkaç dakika sonra yeniden deneyin. Ürününüzü teslim aldıysanız kayıt bir süre sonra kendiliğinden silinir.",
    yedek: "Bu sayfa açılmıyorsa:", yedek_a: "yedek takip sayfası",
    iptal_b: "İşlem iptal edildi", iptal_m: "Bu fişteki tadilat işlemi iptal edildi. Ayrıntı için mağazamızla görüşebilirsiniz.",
    adim: { alindi: "Ürün alındı", terzide: "Terzide", hazir: "Hazır", teslim: "Teslim edildi" },
    durum: { alindi: "Ürününüz teslim alındı", terzide: "Ürününüz terzide", hazir: "Ürününüz hazır", teslim: "Ürününüz teslim edildi" },
    tahmini: "Tahmini teslim",
    hazir_m: "Ürününüzü saat 17:00'den itibaren fişinizle birlikte teslim alabilirsiniz. 14 gün içinde alınmayan ürünlerden mesul değiliz.",
    teslim_m: "Bizi tercih ettiğiniz için teşekkür ederiz.",
    fis: "Fiş", fis_tarihi: "Fiş tarihi", teslim_gunu: "Teslim", islemler: "Yapılacak işlemler", diger_islem: "Diğer tadilat",
  },
  en: {
    guncel: "Current", ozet: "Receipt summary",
    baslik: "Alteration tracking", sube: "Branch", dil: "Language",
    yok_b: "Record not found",
    yok_m: "If your receipt was issued just now, please try again in a few minutes. Once you have collected your item, the record is deleted automatically after a while.",
    yedek: "If this page does not open:", yedek_a: "backup tracking page",
    iptal_b: "Alteration cancelled", iptal_m: "The alteration on this receipt has been cancelled. Please contact our store for details.",
    adim: { alindi: "Receipt issued", terzide: "At the tailor", hazir: "Ready", teslim: "Delivered" },
    durum: { alindi: "We have received your item", terzide: "Your item is at the tailor", hazir: "Your item is ready", teslim: "Your item has been collected" },
    tahmini: "Estimated ready date",
    hazir_m: "You can collect your item with your receipt from 17:00. We are not responsible for items not collected within 14 days.",
    teslim_m: "Thank you for choosing us.",
    fis: "Receipt", fis_tarihi: "Receipt date", teslim_gunu: "Ready on", islemler: "Alterations", diger_islem: "Other alteration",
  },
  ar: {
    guncel: "الحالية", ozet: "ملخص الإيصال",
    baslik: "تتبع التعديل", sube: "الفرع", dil: "اللغة",
    yok_b: "لم يتم العثور على السجل",
    yok_m: "إذا صدر إيصالك للتو، يرجى المحاولة مرة أخرى بعد بضع دقائق. بعد استلام قطعتك يُحذف السجل تلقائيًا بعد فترة.",
    yedek: "إذا لم تفتح هذه الصفحة:", yedek_a: "صفحة التتبع الاحتياطية",
    iptal_b: "تم إلغاء التعديل", iptal_m: "تم إلغاء التعديل الخاص بهذا الإيصال. للتفاصيل يرجى التواصل مع متجرنا.",
    adim: { alindi: "تم إصدار الإيصال", terzide: "لدى الخياط", hazir: "جاهز", teslim: "تم التسليم" },
    durum: { alindi: "استلمنا قطعتك", terzide: "قطعتك لدى الخياط", hazir: "قطعتك جاهزة", teslim: "تم تسليم قطعتك إليك" },
    tahmini: "موعد الجاهزية المتوقع",
    hazir_m: "يمكنك استلام قطعتك مع الإيصال ابتداءً من الساعة 17:00. لسنا مسؤولين عن القطع التي لا تُستلم خلال 14 يومًا.",
    teslim_m: "شكرًا لاختياركم لنا.",
    fis: "الإيصال", fis_tarihi: "تاريخ الإيصال", teslim_gunu: "موعد الاستلام", islemler: "التعديلات", diger_islem: "تعديل آخر",
  },
  ru: {
    guncel: "Сейчас", ozet: "Сводка по чеку",
    baslik: "Статус переделки одежды", sube: "Филиал", dil: "Язык",
    yok_b: "Запись не найдена",
    yok_m: "Если чек выдан только что, попробуйте снова через несколько минут. После получения изделия запись через некоторое время удаляется автоматически.",
    yedek: "Если страница не открывается:", yedek_a: "резервная страница",
    iptal_b: "Переделка одежды отменена", iptal_m: "Работа по этому чеку отменена. Подробности уточните в нашем магазине.",
    adim: { alindi: "Чек выдан", terzide: "У портного", hazir: "Готово", teslim: "Передано клиенту" },
    durum: { alindi: "Мы приняли ваше изделие", terzide: "Ваше изделие у портного", hazir: "Ваше изделие готово", teslim: "Изделие выдано" },
    tahmini: "Ориентировочная дата готовности",
    hazir_m: "Изделие можно забрать с чеком начиная с 17:00. Мы не несём ответственности за изделия, не забранные в течение 14 дней.",
    teslim_m: "Спасибо, что выбрали нас.",
    fis: "Чек", fis_tarihi: "Дата чека", teslim_gunu: "Готово", islemler: "Работы", diger_islem: "Другая работа",
  },
  de: {
    guncel: "Aktuell", ozet: "Belegübersicht",
    baslik: "Status Ihrer Kleidungsänderung", sube: "Filiale", dil: "Sprache",
    yok_b: "Eintrag nicht gefunden",
    yok_m: "Wenn Ihr Beleg gerade erst ausgestellt wurde, versuchen Sie es bitte in ein paar Minuten erneut. Nach der Abholung wird der Eintrag nach einiger Zeit automatisch gelöscht.",
    yedek: "Falls diese Seite nicht lädt:", yedek_a: "Ersatzseite",
    iptal_b: "Kleidungsänderung storniert", iptal_m: "Die Änderung auf diesem Beleg wurde storniert. Für Details wenden Sie sich bitte an unser Geschäft.",
    adim: { alindi: "Beleg ausgestellt", terzide: "Beim Schneider", hazir: "Fertig", teslim: "Übergeben" },
    durum: { alindi: "Wir haben Ihr Kleidungsstück erhalten", terzide: "Ihr Kleidungsstück ist beim Schneider", hazir: "Ihr Kleidungsstück ist fertig", teslim: "Ihr Kleidungsstück wurde abgeholt" },
    tahmini: "Voraussichtlich fertig am",
    hazir_m: "Sie können Ihr Kleidungsstück ab 17:00 Uhr mit Ihrem Beleg abholen. Für Stücke, die nicht innerhalb von 14 Tagen abgeholt werden, übernehmen wir keine Haftung.",
    teslim_m: "Vielen Dank, dass Sie sich für uns entschieden haben.",
    fis: "Beleg", fis_tarihi: "Belegdatum", teslim_gunu: "Fertig am", islemler: "Änderungen", diger_islem: "Sonstige Änderung",
  },
  fr: {
    guncel: "Étape actuelle", ozet: "Résumé du ticket",
    baslik: "Suivi de retouche", sube: "Boutique", dil: "Langue",
    yok_b: "Dossier introuvable",
    yok_m: "Si votre ticket vient d'être émis, réessayez dans quelques minutes. Une fois l'article récupéré, le dossier est supprimé automatiquement après un certain temps.",
    yedek: "Si cette page ne s'ouvre pas :", yedek_a: "page de suivi de secours",
    iptal_b: "Retouche annulée", iptal_m: "La retouche de ce ticket a été annulée. Pour plus de détails, contactez notre boutique.",
    adim: { alindi: "Ticket émis", terzide: "Chez le tailleur", hazir: "Prêt", teslim: "Remis" },
    durum: { alindi: "Nous avons reçu votre article", terzide: "Votre article est chez le tailleur", hazir: "Votre article est prêt", teslim: "Votre article a été récupéré" },
    tahmini: "Date prévue",
    hazir_m: "Vous pouvez récupérer votre article avec votre ticket à partir de 17h00. Nous déclinons toute responsabilité pour les articles non récupérés sous 14 jours.",
    teslim_m: "Merci de nous avoir choisis.",
    fis: "Ticket", fis_tarihi: "Date du ticket", teslim_gunu: "Prêt le", islemler: "Retouches", diger_islem: "Autre retouche",
  },
  es: {
    guncel: "Actual", ozet: "Resumen del recibo",
    baslik: "Seguimiento de arreglos", sube: "Tienda", dil: "Idioma",
    yok_b: "Registro no encontrado",
    yok_m: "Si su recibo se acaba de emitir, vuelva a intentarlo en unos minutos. Después de recoger su prenda, el registro se borra automáticamente al cabo de un tiempo.",
    yedek: "Si esta página no se abre:", yedek_a: "página de seguimiento alternativa",
    iptal_b: "Arreglo cancelado", iptal_m: "El arreglo de este recibo ha sido cancelado. Para más detalles, contacte con nuestra tienda.",
    adim: { alindi: "Recibo emitido", terzide: "En la sastrería", hazir: "Listo", teslim: "Entregado" },
    durum: { alindi: "Hemos recibido su prenda", terzide: "Su prenda está en la sastrería", hazir: "Su prenda está lista", teslim: "Su prenda ha sido entregada" },
    tahmini: "Fecha estimada",
    hazir_m: "Puede recoger su prenda con el recibo a partir de las 17:00. No nos hacemos responsables de las prendas no recogidas en 14 días.",
    teslim_m: "Gracias por elegirnos.",
    fis: "Recibo", fis_tarihi: "Fecha del recibo", teslim_gunu: "Listo el", islemler: "Arreglos", diger_islem: "Otro arreglo",
  },
  zh: {
    guncel: "当前", ozet: "单据摘要",
    baslik: "改衣进度查询", sube: "门店", dil: "语言",
    yok_b: "未找到记录",
    yok_m: "如果您的单据刚刚开出，请几分钟后再试。取走衣物后，记录会在一段时间后自动删除。",
    yedek: "如果此页面无法打开：", yedek_a: "备用查询页面",
    iptal_b: "改衣服务已取消", iptal_m: "此单据对应的改衣服务已取消。详情请联系我们的门店。",
    adim: { alindi: "已开单", terzide: "裁缝处理中", hazir: "已完成", teslim: "已交付" },
    durum: { alindi: "我们已收到您的衣物", terzide: "您的衣物正在裁缝处修改", hazir: "您的衣物已改好", teslim: "您的衣物已取走" },
    tahmini: "预计完成日期",
    hazir_m: "您可以从17:00起凭单据取衣。14天内未取的衣物，本店概不负责。",
    teslim_m: "感谢您的光临。",
    fis: "单据", fis_tarihi: "开单日期", teslim_gunu: "取衣日期", islemler: "修改项目", diger_islem: "其他修改",
  },
  hi: {
    guncel: "वर्तमान", ozet: "रसीद का सारांश",
    baslik: "कपड़े में बदलाव की स्थिति", sube: "शाखा", dil: "भाषा",
    yok_b: "रिकॉर्ड नहीं मिला",
    yok_m: "अगर आपकी रसीद अभी-अभी बनी है, तो कुछ मिनट बाद फिर कोशिश करें। कपड़ा ले जाने के कुछ समय बाद रिकॉर्ड अपने-आप हट जाता है।",
    yedek: "अगर यह पेज नहीं खुलता:", yedek_a: "वैकल्पिक पेज",
    iptal_b: "कपड़े में बदलाव का काम रद्द हो गया", iptal_m: "इस रसीद का काम रद्द कर दिया गया है। जानकारी के लिए हमारी दुकान से संपर्क करें।",
    adim: { alindi: "रसीद बनी", terzide: "दर्ज़ी के पास", hazir: "तैयार", teslim: "सौंप दिया गया" },
    durum: { alindi: "हमें आपका कपड़ा मिल गया है", terzide: "आपका कपड़ा दर्ज़ी के पास है", hazir: "आपका कपड़ा तैयार है", teslim: "आपका कपड़ा ले जाया गया" },
    tahmini: "अनुमानित तैयार होने की तारीख",
    hazir_m: "आप 17:00 बजे से रसीद दिखाकर अपना कपड़ा ले सकते हैं। 14 दिन में न ले जाए गए कपड़ों की ज़िम्मेदारी हमारी नहीं है।",
    teslim_m: "हमें चुनने के लिए धन्यवाद।",
    fis: "रसीद", fis_tarihi: "रसीद की तारीख", teslim_gunu: "तैयार होने की तारीख", islemler: "काम", diger_islem: "अन्य बदलाव",
  },
  fa: {
    guncel: "فعلی", ozet: "خلاصهٔ رسید",
    baslik: "پیگیری اصلاح لباس", sube: "شعبه", dil: "زبان",
    yok_b: "سابقه‌ای پیدا نشد",
    yok_m: "اگر رسید شما همین حالا صادر شده، چند دقیقه دیگر دوباره امتحان کنید. پس از تحویل گرفتن لباس، سابقه پس از مدتی خودکار حذف می‌شود.",
    yedek: "اگر این صفحه باز نشد:", yedek_a: "صفحهٔ پیگیری جایگزین",
    iptal_b: "اصلاح لباس لغو شد", iptal_m: "کار اصلاح این رسید لغو شده است. برای جزئیات با فروشگاه ما تماس بگیرید.",
    adim: { alindi: "رسید صادر شد", terzide: "نزد خیاط", hazir: "آماده", teslim: "تحویل داده شد" },
    durum: { alindi: "لباس شما را دریافت کردیم", terzide: "لباس شما نزد خیاط است", hazir: "لباس شما آماده است", teslim: "لباس شما تحویل داده شد" },
    tahmini: "تاریخ تقریبی آماده شدن",
    hazir_m: "می‌توانید از ساعت 17:00 با رسید خود لباس را تحویل بگیرید. مسئولیت لباس‌هایی که ظرف 14 روز تحویل گرفته نشوند با ما نیست.",
    teslim_m: "از اینکه ما را انتخاب کردید سپاسگزاریم.",
    fis: "رسید", fis_tarihi: "تاریخ رسید", teslim_gunu: "تاریخ تحویل", islemler: "کارهای اصلاح", diger_islem: "اصلاح دیگر",
  },
  ro: {
    guncel: "Acum", ozet: "Rezumatul bonului",
    baslik: "Urmărirea ajustării", sube: "Magazin", dil: "Limbă",
    yok_b: "Înregistrare negăsită",
    yok_m: "Dacă bonul tocmai a fost emis, încercați din nou peste câteva minute. După ce ridicați articolul, înregistrarea se șterge automat după un timp.",
    yedek: "Dacă această pagină nu se deschide:", yedek_a: "pagina de rezervă",
    iptal_b: "Ajustare anulată", iptal_m: "Ajustarea de pe acest bon a fost anulată. Pentru detalii, contactați magazinul nostru.",
    adim: { alindi: "Bon emis", terzide: "La croitor", hazir: "Gata", teslim: "Predat clientului" },
    durum: { alindi: "Am primit articolul dumneavoastră", terzide: "Articolul dumneavoastră este la croitor", hazir: "Articolul dumneavoastră este gata", teslim: "Articolul a fost ridicat" },
    tahmini: "Data estimată",
    hazir_m: "Puteți ridica articolul cu bonul începând cu ora 17:00. Nu ne asumăm răspunderea pentru articolele neridicate în 14 zile.",
    teslim_m: "Vă mulțumim că ne-ați ales.",
    fis: "Bon", fis_tarihi: "Data bonului", teslim_gunu: "Gata la", islemler: "Ajustări", diger_islem: "Altă ajustare",
  },
  el: {
    guncel: "Τώρα", ozet: "Σύνοψη απόδειξης",
    baslik: "Παρακολούθηση μεταποίησης", sube: "Κατάστημα", dil: "Γλώσσα",
    yok_b: "Η εγγραφή δεν βρέθηκε",
    yok_m: "Αν η απόδειξή σας εκδόθηκε μόλις τώρα, δοκιμάστε ξανά σε λίγα λεπτά. Αφού παραλάβετε το ρούχο, η εγγραφή διαγράφεται αυτόματα μετά από λίγο καιρό.",
    yedek: "Αν η σελίδα δεν ανοίγει:", yedek_a: "εφεδρική σελίδα",
    iptal_b: "Η μεταποίηση ακυρώθηκε", iptal_m: "Η μεταποίηση αυτής της απόδειξης ακυρώθηκε. Για λεπτομέρειες επικοινωνήστε με το κατάστημά μας.",
    adim: { alindi: "Εκδόθηκε απόδειξη", terzide: "Στον ράφτη", hazir: "Έτοιμο", teslim: "Παραδόθηκε" },
    durum: { alindi: "Παραλάβαμε το ρούχο σας", terzide: "Το ρούχο σας είναι στον ράφτη", hazir: "Το ρούχο σας είναι έτοιμο", teslim: "Το ρούχο σας παραδόθηκε" },
    tahmini: "Εκτιμώμενη ημερομηνία",
    hazir_m: "Μπορείτε να παραλάβετε το ρούχο σας με την απόδειξη από τις 17:00. Δεν φέρουμε ευθύνη για ρούχα που δεν παραλαμβάνονται εντός 14 ημερών.",
    teslim_m: "Σας ευχαριστούμε που μας προτιμήσατε.",
    fis: "Απόδειξη", fis_tarihi: "Ημερομηνία απόδειξης", teslim_gunu: "Έτοιμο στις", islemler: "Μεταποιήσεις", diger_islem: "Άλλη μεταποίηση",
  },
  bg: {
    guncel: "Сега", ozet: "Обобщение на бележката",
    baslik: "Проследяване на поправка", sube: "Магазин", dil: "Език",
    yok_b: "Записът не е намерен",
    yok_m: "Ако бележката ви току-що е издадена, опитайте отново след няколко минути. След като вземете дрехата, записът се изтрива автоматично след известно време.",
    yedek: "Ако страницата не се отваря:", yedek_a: "резервна страница",
    iptal_b: "Поправката е отменена", iptal_m: "Поправката по тази бележка е отменена. За подробности се свържете с нашия магазин.",
    adim: { alindi: "Издадена бележка", terzide: "При шивача", hazir: "Готово", teslim: "Предадено" },
    durum: { alindi: "Получихме вашата дреха", terzide: "Дрехата ви е при шивача", hazir: "Дрехата ви е готова", teslim: "Дрехата ви е взета" },
    tahmini: "Очаквана дата",
    hazir_m: "Можете да вземете дрехата си с бележката от 17:00 ч. Не носим отговорност за дрехи, които не са взети в рамките на 14 дни.",
    teslim_m: "Благодарим ви, че ни избрахте.",
    fis: "Бележка", fis_tarihi: "Дата на бележката", teslim_gunu: "Готово на", islemler: "Поправки", diger_islem: "Друга поправка",
  },
};

// Sütun sırası DILLER ile aynı: tr, en, ar, ru, de, fr, es, zh, hi, fa, ro, el, bg
const URUN = {
  pantolon: ["Pantolon", "Trousers", "بنطال", "Брюки", "Hose", "Pantalon", "Pantalón", "裤子", "पतलून", "شلوار", "Pantaloni", "Παντελόνι", "Панталон"],
  ceket: ["Ceket", "Jacket", "سترة", "Пиджак", "Sakko", "Veste", "Chaqueta", "西装外套", "जैकेट", "کت", "Sacou", "Σακάκι", "Сако"],
  gomlek: ["Gömlek", "Shirt", "قميص", "Рубашка", "Hemd", "Chemise", "Camisa", "衬衫", "कमीज़", "پیراهن", "Cămașă", "Πουκάμισο", "Риза"],
  yelek: ["Yelek", "Waistcoat", "صديري", "Жилет", "Weste", "Gilet", "Chaleco", "马甲", "वेस्टकोट", "جلیقه", "Vestă", "Γιλέκο", "Елек"],
  takim: ["Takım elbise", "Suit", "بدلة", "Костюм", "Anzug", "Costume", "Traje", "西装套装", "सूट", "کت و شلوار", "Costum", "Κοστούμι", "Костюм"],
  etek: ["Etek", "Skirt", "تنورة", "Юбка", "Rock", "Jupe", "Falda", "裙子", "स्कर्ट", "دامن", "Fustă", "Φούστα", "Пола"],
  elbise: ["Elbise", "Dress", "فستان", "Платье", "Kleid", "Robe", "Vestido", "连衣裙", "ड्रेस", "پیراهن زنانه", "Rochie", "Φόρεμα", "Рокля"],
  mont: ["Mont", "Coat", "معطف", "Куртка", "Jacke", "Blouson", "Abrigo corto", "夹克外套", "कोट", "کاپشن", "Geacă", "Μπουφάν", "Яке"],
  palto: ["Palto", "Overcoat", "معطف طويل", "Пальто", "Mantel", "Manteau", "Abrigo", "大衣", "ओवरकोट", "پالتو", "Palton", "Παλτό", "Палто"],
  bluz: ["Bluz", "Blouse", "بلوزة", "Блузка", "Bluse", "Chemisier", "Blusa", "女式衬衫", "ब्लाउज़", "بلوز", "Bluză", "Μπλούζα", "Блуза"],
  kot: ["Kot pantolon", "Jeans", "جينز", "Джинсы", "Jeans", "Jean", "Vaqueros", "牛仔裤", "जींस", "شلوار جین", "Blugi", "Τζιν", "Дънки"],
  tisort: ["Tişört", "T-shirt", "تي شيرت", "Футболка", "T-Shirt", "T-shirt", "Camiseta", "T恤", "टी-शर्ट", "تی‌شرت", "Tricou", "Μπλουζάκι", "Тениска"],
  diger: ["Diğer ürün", "Other item", "قطعة أخرى", "Другое изделие", "Sonstiges", "Autre article", "Otra prenda", "其他衣物", "अन्य कपड़ा", "لباس دیگر", "Alt articol", "Άλλο ρούχο", "Друга дреха"],
};

// Katalog işlemleri (taa-terzi islemler.json kodları)
const KISALT = ["Paça kısaltma", "Hem shortened", "تقصير الطرف السفلي", "Укорачивание низа брюк", "Saum kürzen", "Ourlet raccourci", "Bajo acortado", "裤脚改短", "मोहरी छोटी करना", "کوتاه کردن پاچه", "Scurtare tiv", "Κόντεμα στρίφωμα", "Скъсяване на крачолите"];
const KOL_KISALT = ["Kol kısaltma", "Sleeves shortened", "تقصير الأكمام", "Укорачивание рукавов", "Ärmel kürzen", "Manches raccourcies", "Mangas acortadas", "袖子改短", "आस्तीन छोटी करना", "کوتاه کردن آستین", "Scurtare mâneci", "Κόντεμα μανικιών", "Скъсяване на ръкавите"];
const DARALT = ["Daraltma", "Taken in", "تضييق", "Ушивание", "Enger machen", "Resserrage", "Estrechado", "收窄", "तंग करना", "تنگ کردن", "Strâmtare", "Στένεμα", "Стесняване"];
const DUGME = ["Düğme kaydırma", "Button moved", "نقل الأزرار", "Перешив пуговиц", "Knopf versetzen", "Bouton déplacé", "Botón desplazado", "移动纽扣", "बटन खिसकाना", "جابه‌جایی دکمه", "Mutare nasture", "Μετακίνηση κουμπιού", "Преместване на копче"];
const BEDEN = ["Bir beden daraltma", "Taken in one size", "تصغير مقاس واحد", "Ушивание на один размер", "Um eine Größe enger", "Resserrage d'une taille", "Estrechado una talla", "改小一个尺码", "एक साइज़ छोटा करना", "یک سایز تنگ کردن", "Strâmtare cu o mărime", "Στένεμα κατά ένα νούμερο", "Стесняване с един размер"];
const ISLEM = {
  "pant-paca-kisalt": ["Paça kısaltma", ...KISALT.slice(1)],
  "pant-bacak-daralt": ["Bacak daraltma", "Leg taper", "تضييق الساق", "Сужение брючин", "Bein enger machen", "Jambes resserrées", "Pierna estrechada", "裤腿收窄", "पैर तंग करना", "تنگ کردن پاچه", "Strâmtare crac", "Στένεμα γάμπας", "Стесняване на крачолите"],
  "pant-bel-daralt": ["Bel daraltma", "Waist taken in", "تضييق الخصر", "Ушивание пояса", "Bund enger machen", "Taille resserrée", "Cintura estrechada", "腰围收窄", "कमर तंग करना", "تنگ کردن کمر", "Strâmtare talie", "Στένεμα μέσης", "Стесняване на талията"],
  "pant-bacak-payi-ac": ["Bacak payı açma", "Leg let out", "توسيع الساق", "Расширение брючин", "Bein auslassen", "Jambes élargies", "Pierna ensanchada", "裤腿放宽", "पैर ढीला करना", "گشاد کردن پاچه", "Lărgire crac", "Φάρδεμα γάμπας", "Разширяване на крачолите"],
  "pant-paca-payi-ac": ["Paça payı açma", "Hem let down", "تطويل الطرف السفلي", "Удлинение низа", "Saum auslassen", "Ourlet rallongé", "Bajo alargado", "裤脚放长", "मोहरी लंबी करना", "بلند کردن پاچه", "Lungire tiv", "Μάκρεμα στρίφωμα", "Удължаване на крачолите"],
  "pant-bel-payi-ac": ["Bel payı açma", "Waist let out", "توسيع الخصر", "Расширение пояса", "Bund auslassen", "Taille élargie", "Cintura ensanchada", "腰围放宽", "कमर ढीली करना", "گشاد کردن کمر", "Lărgire talie", "Φάρδεμα μέσης", "Разширяване на талията"],
  "pant-fermuar": ["Fermuar değişimi", "Zip replacement", "تغيير السحاب", "Замена молнии", "Reißverschluss ersetzen", "Remplacement de fermeture éclair", "Cambio de cremallera", "更换拉链", "ज़िप बदलना", "تعویض زیپ", "Înlocuire fermoar", "Αλλαγή φερμουάρ", "Смяна на цип"],
  "pant-salvar-daralt": ["Şalvarlık için daraltma", "Seat taken in", "تضييق منطقة الجلوس", "Ушивание в шаге", "Gesäß enger machen", "Fond resserré", "Tiro estrechado", "裆部收窄", "सीट तंग करना", "تنگ کردن فاق", "Strâmtare la șezut", "Στένεμα καβάλου", "Стесняване в седалището"],
  "cek-etek-kisalt": ["Etek boyu kısaltma", "Jacket length shortened", "تقصير طول السترة", "Укорачивание длины", "Länge kürzen", "Longueur raccourcie", "Largo acortado", "衣长改短", "लंबाई छोटी करना", "کوتاه کردن قد کت", "Scurtare lungime", "Κόντεμα μήκους", "Скъсяване на дължината"],
  "cek-kol-kisalt": ["Kol boyu kısaltma", ...KOL_KISALT.slice(1)],
  "cek-sirt-daralt": ["Sırt daraltma", "Back taken in", "تضييق الظهر", "Ушивание спинки", "Rücken enger machen", "Dos resserré", "Espalda estrechada", "后背收窄", "पीठ तंग करना", "تنگ کردن پشت", "Strâmtare spate", "Στένεμα πλάτης", "Стесняване на гърба"],
  "cek-kol-daralt": ["Kol daraltma", "Sleeves narrowed", "تضييق الأكمام", "Сужение рукавов", "Ärmel enger machen", "Manches resserrées", "Mangas estrechadas", "袖子收窄", "आस्तीन तंग करना", "تنگ کردن آستین", "Strâmtare mâneci", "Στένεμα μανικιών", "Стесняване на ръкавите"],
  "cek-omuz-kol-kisalt": ["Omuzdan kol kısaltma", "Sleeves shortened from the shoulder", "تقصير الأكمام من الكتف", "Укорачивание рукавов от плеча", "Ärmel an der Schulter kürzen", "Manches raccourcies par l'épaule", "Mangas acortadas desde el hombro", "从肩部改短袖子", "कंधे से आस्तीन छोटी करना", "کوتاه کردن آستین از سرشانه", "Scurtare mâneci de la umăr", "Κόντεμα μανικιών από τον ώμο", "Скъсяване на ръкавите от рамото"],
  "cek-dugme-kaydir": DUGME,
  "cek-dugme-saglam": ["Düğme sağlamlaştırma", "Buttons secured", "تثبيت الأزرار", "Укрепление пуговиц", "Knöpfe befestigen", "Boutons consolidés", "Botones reforzados", "加固纽扣", "बटन मज़बूत करना", "محکم کردن دکمه‌ها", "Întărire nasturi", "Στερέωση κουμπιών", "Укрепване на копчетата"],
  "cek-vatka-kisalt": ["Omuz vatkası kısaltma", "Shoulder pads reduced", "تقليل حشوة الكتف", "Уменьшение плечевых накладок", "Schulterpolster verkleinern", "Épaulettes réduites", "Hombreras reducidas", "垫肩改小", "कंधे का पैड छोटा करना", "کوچک کردن سرشانه", "Micșorare umerași", "Μείωση βάτας", "Намаляване на подплънките"],
  "cek-yirtmac-kapat": ["Yırtmaç kapama", "Vent closed", "إغلاق الشق", "Зашивание шлицы", "Schlitz schließen", "Fente fermée", "Abertura cerrada", "缝合开衩", "चाक बंद करना", "بستن چاک", "Închidere șliț", "Κλείσιμο σκισίματος", "Зашиване на цепката"],
  "gom-kol-kisalt": KOL_KISALT,
  "gom-daralt": DARALT,
  "gom-dugme-kaydir": DUGME,
  "gom-yaka-sabitle": ["Yaka sabitleme", "Collar secured", "تثبيت الياقة", "Закрепление воротника", "Kragen fixieren", "Col fixé", "Cuello fijado", "固定领子", "कॉलर ठीक करना", "ثابت کردن یقه", "Fixare guler", "Στερέωση γιακά", "Закрепване на яката"],
  "yel-daralt": DARALT,
  "yel-omuz-kisalt": ["Omuzdan kısaltma", "Shortened at the shoulder", "تقصير من الكتف", "Укорачивание по плечу", "An der Schulter kürzen", "Raccourci par l'épaule", "Acortado desde el hombro", "从肩部改短", "कंधे से छोटा करना", "کوتاه کردن از سرشانه", "Scurtare de la umăr", "Κόντεμα από τον ώμο", "Скъсяване от рамото"],
  "yel-kulak-kapat": ["Koltuk altı kulak kapama", "Armhole closed", "تضييق فتحة الإبط", "Ушивание проймы", "Armloch verkleinern", "Emmanchure resserrée", "Sisa cerrada", "腋下收紧", "बगल तंग करना", "تنگ کردن حلقهٔ آستین", "Strâmtare la subraț", "Στένεμα μασχάλης", "Стесняване под мишниците"],
  "takim-beden-daralt": ["Komple bir beden daraltma", ...BEDEN.slice(1)],
  "pant-beden-daralt": ["Komple bir beden daraltma", ...BEDEN.slice(1)],
  "cek-beden-daralt": BEDEN,
  "yel-beden-daralt": BEDEN,
  "yel-genislet-parca": ["Genişletme (parça ekleyerek)", "Let out (with added panel)", "توسيع (بإضافة قطعة)", "Расширение (вставкой)", "Erweitern (mit Einsatz)", "Élargissement (par ajout de pièce)", "Ensanchado (con pieza añadida)", "加布放宽", "कपड़ा जोड़कर ढीला करना", "گشاد کردن (با افزودن تکه)", "Lărgire (cu bucată adăugată)", "Φάρδεμα (με πρόσθετο κομμάτι)", "Разширяване (с добавена вложка)"],
  "gom-etek-kisalt": ["Etek kısaltma", "Hem shortened", "تقصير الطول", "Укорачивание низа", "Saum kürzen", "Bas raccourci", "Bajo acortado", "下摆改短", "नीचे से छोटा करना", "کوتاه کردن دامن پیراهن", "Scurtare tiv", "Κόντεμα στρίφωμα", "Скъсяване на долния край"],
  "takim-3parca-beden-daralt": ["Takım elbise (ceket+pantolon+yelek) bir beden daraltma", "Three-piece suit taken in one size", "تصغير بدلة من ثلاث قطع مقاسًا واحدًا", "Ушивание костюма-тройки на один размер", "Dreiteiliger Anzug um eine Größe enger", "Costume trois pièces resserré d'une taille", "Traje de tres piezas estrechado una talla", "三件套西装改小一个尺码", "थ्री-पीस सूट एक साइज़ छोटा करना", "یک سایز تنگ کردن کت و شلوار سه‌تکه", "Costum din trei piese strâmtat cu o mărime", "Τρίτεμπο κοστούμι στένεμα κατά ένα νούμερο", "Костюм от три части, стеснен с един размер"],
};

const VARYANT = {
  duz: ["Düz paça", "Plain hem", "طرف عادي", "Без отворота", "Ohne Aufschlag", "Ourlet simple", "Bajo sencillo", "平脚", "सादी मोहरी", "پاچهٔ ساده", "Tiv simplu", "Απλό στρίφωμα", "Без маншет"],
  duble: ["Duble paça", "Turn-up hem", "طرف مزدوج", "С отворотом", "Mit Aufschlag", "Ourlet à revers", "Bajo con vuelta", "翻边裤脚", "मुड़ी हुई मोहरी", "پاچهٔ برگردان", "Tiv cu manșetă", "Ρεβέρ", "С маншет"],
};

const SIRA = DILLER.map(([d]) => d);
const al = (tablo, anahtar, dil) => {
  const s = tablo[anahtar];
  return s ? s[SIRA.indexOf(dil)] || s[0] : null;
};
export const urunAdi = (u, dil) => al(URUN, u, dil) || al(URUN, "diger", dil);
export const islemAdi = (i, dil) => (i ? al(ISLEM, i, dil) : null);
export const varyantAdi = (v, dil) => (v ? al(VARYANT, v, dil) : null);
/** Testler için: her tablo satırı tam mı? */
export const TABLOLAR = { URUN, ISLEM, VARYANT };

/**
 * Dil seçimi: önce ?dil= (kullanıcı çekmeceden seçtiyse), sonra hatırlanan seçim (çerez), sonra telefonun sistem dili
 * (tarayıcı bunu Accept-Language olarak gönderir; q ağırlıklarına göre sıralanır), yoksa Türkçe.
 * "zh-CN", "zh-Hans-TW", "pt-BR" gibi bölgeli kodlarda ana dil alınır; "iw" eski koddur, kullanılmaz.
 */
export function dilSec(sorgu, kabul, cerez = "") {
  if (sorgu && METIN[sorgu]) return sorgu;
  // Daha önce çekmeceden seçilen dil (çerez "dil", yalnız dil kodu; kişisel veri yok).
  const c = /(?:^|;\s*)dil=([a-z]{2})(?:;|$)/.exec(String(cerez || ""));
  if (c && METIN[c[1]]) return c[1];
  const adaylar = String(kabul || "")
    .split(",")
    .map((p, sira) => {
      const [etiket, ...ek] = p.trim().split(";");
      const q = ek.map((e) => e.trim()).find((e) => e.startsWith("q="));
      const agirlik = q ? Number(q.slice(2)) : 1;
      return { d: etiket.split("-")[0].toLowerCase(), q: Number.isFinite(agirlik) ? agirlik : 0, sira };
    })
    .filter((a) => a.d && a.q > 0)
    .sort((a, b) => b.q - a.q || a.sira - b.sira);
  for (const a of adaylar) if (METIN[a.d]) return a.d;
  return "tr";
}
