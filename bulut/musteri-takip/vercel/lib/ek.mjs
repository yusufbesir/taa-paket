// TAA müşteri takip: v3 ekleri (kargo, şube kartı, adım açıklamaları).
// Şube bilgisi kamuya açık işletme bilgisidir (fiş başlığında basılı olan); kayıtta yalnız şube kodu gelir.
// Kargo firmaları ve takip bağlantıları burada; takip bağlantısı kalıbı doğrulanmamış firmada yalnız ana sayfa açılır.
// Yazan: [Claude Opus 5.5, TerziTakip başlığı, 2026-11-26].

/** Şube kodu → kamuya açık bilgi. Adresi girilmemiş şubede yalnız ad gösterilir. */
export const SUBELER = {
  MRK01: {
    ad: "İstanbul Merkez Mağaza",
    adres: "Turgut Özal Mah. 68. Sk. No:12, E-5 Karayolu Yan Yol Üzeri, Esenyurt / İstanbul",
    tel: "+902126900202", tel_g: "0212 690 02 02", saat: "09:30–21:30",
  },
  PND01: { ad: "İstanbul Pendik Mağaza" },
  UMR01: { ad: "İstanbul Ümraniye Mağaza" },
  ANK01: { ad: "Ankara Mağaza" },
  BRS01: { ad: "Bursa Mağaza" },
  SKR01: { ad: "Sakarya Mağaza" },
  DYB01: { ad: "Diyarbakır Mağaza" },
  SVS01: { ad: "Sivas Mağaza" },
  GZT01: { ad: "Gaziantep Mağaza" },
  URF01: { ad: "Şanlıurfa Mağaza" },
  VAN01: { ad: "Van Mağaza" },
  SRT01: { ad: "Siirt Mağaza" },
  ELZ01: { ad: "Elazığ Mağaza" },
  IZM01: { ad: "İzmir Mağaza" },
  MUS01: { ad: "Muş Mağaza" },
};

/** Kargo firması anahtarı → ad ve takip bağlantısı (n: takip no, encodeURIComponent'li). */
export const KARGO = {
  yurtici: { ad: "Yurtiçi Kargo", url: (n) => `https://www.yurticikargo.com/tr/online-servisler/gonderi-sorgula?code=${n}` },
  aras: { ad: "Aras Kargo", url: (n) => `https://kargotakip.araskargo.com.tr/mainpage.aspx?code=${n}` },
  mng: { ad: "MNG Kargo", url: () => "https://www.dhlecommerce.com.tr/" },
  ptt: { ad: "PTT Kargo", url: (n) => `https://gonderitakip.ptt.gov.tr/Track/Verify?q=${n}` },
  surat: { ad: "Sürat Kargo", url: () => "https://www.suratkargo.com.tr/" },
  ups: { ad: "UPS", url: (n) => `https://www.ups.com/track?loc=tr_TR&tracknum=${n}` },
  dhl: { ad: "DHL", url: (n) => `https://www.dhl.com/tr-tr/home/tracking.html?tracking-id=${n}&submit=1` },
  hepsijet: { ad: "HepsiJET", url: () => "https://www.hepsijet.com/" },
  kolaygelsin: { ad: "Kolay Gelsin", url: () => "https://www.kolaygelsin.com/" },
  trendyol: { ad: "Trendyol Express", url: () => "https://www.trendyolexpress.com/" },
  diger: { ad: "", url: () => "" },
};
export const KARGO_DURUM = ["hazirlaniyor", "yolda", "teslim"];

// Dil başına ek metinler. acik: adım altı açıklaması; kd: kargo durumu başlığı.
export const EK = {
  tr: {
    acik: { alindi: "Ürününüz mağazamıza bırakıldı", terzide: "Ustamız tadilatınızı yapıyor", hazir: "Mağazadan teslim alabilirsiniz", teslim: "Ürününüz size ulaştı", kargo: "Ürününüz kargo ile yola çıktı", kargo_h: "Paketiniz hazırlanıyor" },
    adim_kargo: "Kargolandı", adim_kargo_h: "Kargoya hazırlanıyor",
    kd: { hazirlaniyor: "Ürününüz kargoya hazırlanıyor", yolda: "Ürününüz kargoya verildi", teslim: "Kargonuz teslim edildi" },
    kargo: "Kargo bilgisi", firma: "Kargo firması", takip_no: "Takip numarası", kargo_tarihi: "Kargoya veriliş", varis: "Gönderim yeri",
    takip_et: "Kargoyu takip et", kargo_m: "Takip numarası kargoya verildiğinde burada görünecek.",
    teslim_yeri: "Teslim alma adresi", sube_b: "Şubemiz", saatler: "Çalışma saatleri", ara: "Şubeyi ara", yol: "Yol tarifi", guncelleme: "Son güncelleme",
  },
  en: {
    acik: { alindi: "Your item was left at our store", terzide: "Our tailor is working on your alteration", hazir: "You can collect it at the store", teslim: "Your item has reached you", kargo: "Your item is on its way by courier", kargo_h: "Your parcel is being prepared" },
    adim_kargo: "Shipped", adim_kargo_h: "Preparing shipment",
    kd: { hazirlaniyor: "Your item is being prepared for shipping", yolda: "Your item has been shipped", teslim: "Your parcel has been delivered" },
    kargo: "Shipping details", firma: "Courier", takip_no: "Tracking number", kargo_tarihi: "Shipped on", varis: "Destination",
    takip_et: "Track parcel", kargo_m: "The tracking number will appear here once the parcel is shipped.",
    teslim_yeri: "Pickup address", sube_b: "Our store", saatler: "Opening hours", ara: "Call the store", yol: "Directions", guncelleme: "Last updated",
  },
  ar: {
    acik: { alindi: "تم تسليم قطعتك إلى متجرنا", terzide: "خياطنا يعمل على التعديل", hazir: "يمكنك استلامها من المتجر", teslim: "وصلت قطعتك إليك", kargo: "قطعتك في الطريق مع شركة الشحن", kargo_h: "يتم تجهيز طردك" },
    adim_kargo: "تم الشحن", adim_kargo_h: "قيد التجهيز للشحن",
    kd: { hazirlaniyor: "يتم تجهيز قطعتك للشحن", yolda: "تم شحن قطعتك", teslim: "تم تسليم طردك" },
    kargo: "معلومات الشحن", firma: "شركة الشحن", takip_no: "رقم التتبع", kargo_tarihi: "تاريخ الشحن", varis: "الوجهة",
    takip_et: "تتبع الشحنة", kargo_m: "سيظهر رقم التتبع هنا بعد شحن الطرد.",
    teslim_yeri: "عنوان الاستلام", sube_b: "متجرنا", saatler: "ساعات العمل", ara: "اتصل بالمتجر", yol: "الاتجاهات", guncelleme: "آخر تحديث",
  },
  ru: {
    acik: { alindi: "Изделие принято в нашем магазине", terzide: "Наш портной выполняет переделку", hazir: "Можно забрать в магазине", teslim: "Изделие у вас", kargo: "Изделие отправлено курьерской службой", kargo_h: "Посылка готовится к отправке" },
    adim_kargo: "Отправлено", adim_kargo_h: "Готовится к отправке",
    kd: { hazirlaniyor: "Изделие готовится к отправке", yolda: "Изделие отправлено", teslim: "Посылка доставлена" },
    kargo: "Доставка", firma: "Служба доставки", takip_no: "Трек-номер", kargo_tarihi: "Дата отправки", varis: "Куда",
    takip_et: "Отследить посылку", kargo_m: "Трек-номер появится здесь после отправки.",
    teslim_yeri: "Адрес получения", sube_b: "Наш магазин", saatler: "Часы работы", ara: "Позвонить в магазин", yol: "Как добраться", guncelleme: "Обновлено",
  },
  de: {
    acik: { alindi: "Ihr Artikel wurde in unserem Geschäft abgegeben", terzide: "Unser Schneider arbeitet an Ihrer Änderung", hazir: "Abholbereit im Geschäft", teslim: "Ihr Artikel ist bei Ihnen", kargo: "Ihr Artikel ist mit dem Paketdienst unterwegs", kargo_h: "Ihr Paket wird vorbereitet" },
    adim_kargo: "Versandt", adim_kargo_h: "Versand wird vorbereitet",
    kd: { hazirlaniyor: "Ihr Artikel wird für den Versand vorbereitet", yolda: "Ihr Artikel wurde versandt", teslim: "Ihr Paket wurde zugestellt" },
    kargo: "Versand", firma: "Paketdienst", takip_no: "Sendungsnummer", kargo_tarihi: "Versanddatum", varis: "Zielort",
    takip_et: "Sendung verfolgen", kargo_m: "Die Sendungsnummer erscheint hier, sobald das Paket versandt ist.",
    teslim_yeri: "Abholadresse", sube_b: "Unser Geschäft", saatler: "Öffnungszeiten", ara: "Geschäft anrufen", yol: "Route", guncelleme: "Zuletzt aktualisiert",
  },
  fr: {
    acik: { alindi: "Votre article a été déposé en boutique", terzide: "Notre tailleur réalise votre retouche", hazir: "À retirer en boutique", teslim: "Votre article vous a été remis", kargo: "Votre article est en route par transporteur", kargo_h: "Votre colis est en préparation" },
    adim_kargo: "Expédié", adim_kargo_h: "Préparation de l’envoi",
    kd: { hazirlaniyor: "Votre article est en préparation pour l’envoi", yolda: "Votre article a été expédié", teslim: "Votre colis a été livré" },
    kargo: "Livraison", firma: "Transporteur", takip_no: "Numéro de suivi", kargo_tarihi: "Expédié le", varis: "Destination",
    takip_et: "Suivre le colis", kargo_m: "Le numéro de suivi apparaîtra ici dès l’expédition.",
    teslim_yeri: "Adresse de retrait", sube_b: "Notre boutique", saatler: "Horaires", ara: "Appeler la boutique", yol: "Itinéraire", guncelleme: "Dernière mise à jour",
  },
  es: {
    acik: { alindi: "Su prenda se dejó en nuestra tienda", terzide: "Nuestro sastre está haciendo el arreglo", hazir: "Puede recogerla en la tienda", teslim: "Su prenda ya está con usted", kargo: "Su prenda va en camino con la mensajería", kargo_h: "Estamos preparando su paquete" },
    adim_kargo: "Enviado", adim_kargo_h: "Preparando el envío",
    kd: { hazirlaniyor: "Su prenda se está preparando para el envío", yolda: "Su prenda ha sido enviada", teslim: "Su paquete ha sido entregado" },
    kargo: "Envío", firma: "Mensajería", takip_no: "Número de seguimiento", kargo_tarihi: "Enviado el", varis: "Destino",
    takip_et: "Seguir el envío", kargo_m: "El número de seguimiento aparecerá aquí cuando se envíe el paquete.",
    teslim_yeri: "Dirección de recogida", sube_b: "Nuestra tienda", saatler: "Horario", ara: "Llamar a la tienda", yol: "Cómo llegar", guncelleme: "Última actualización",
  },
  zh: {
    acik: { alindi: "您的衣物已送达本店", terzide: "裁缝师正在为您修改", hazir: "可到店领取", teslim: "衣物已交到您手中", kargo: "衣物已由快递寄出", kargo_h: "正在打包" },
    adim_kargo: "已发货", adim_kargo_h: "准备发货",
    kd: { hazirlaniyor: "您的衣物正在准备发货", yolda: "您的衣物已发货", teslim: "您的包裹已送达" },
    kargo: "物流信息", firma: "快递公司", takip_no: "运单号", kargo_tarihi: "发货日期", varis: "目的地",
    takip_et: "查询物流", kargo_m: "发货后运单号将显示在这里。",
    teslim_yeri: "取件地址", sube_b: "本店", saatler: "营业时间", ara: "致电门店", yol: "路线", guncelleme: "最后更新",
  },
  hi: {
    acik: { alindi: "आपका कपड़ा हमारी दुकान पर जमा हुआ", terzide: "हमारे दर्ज़ी बदलाव कर रहे हैं", hazir: "दुकान से ले सकते हैं", teslim: "कपड़ा आपको मिल गया", kargo: "कपड़ा कूरियर से रास्ते में है", kargo_h: "पार्सल तैयार हो रहा है" },
    adim_kargo: "भेज दिया गया", adim_kargo_h: "भेजने की तैयारी",
    kd: { hazirlaniyor: "आपका कपड़ा भेजने के लिए तैयार हो रहा है", yolda: "आपका कपड़ा भेज दिया गया", teslim: "आपका पार्सल पहुँच गया" },
    kargo: "शिपिंग जानकारी", firma: "कूरियर कंपनी", takip_no: "ट्रैकिंग नंबर", kargo_tarihi: "भेजने की तारीख", varis: "गंतव्य",
    takip_et: "पार्सल ट्रैक करें", kargo_m: "पार्सल भेजे जाने पर ट्रैकिंग नंबर यहाँ दिखेगा।",
    teslim_yeri: "लेने का पता", sube_b: "हमारी दुकान", saatler: "खुलने का समय", ara: "दुकान को कॉल करें", yol: "रास्ता", guncelleme: "अंतिम अपडेट",
  },
  fa: {
    acik: { alindi: "لباس شما در فروشگاه ما تحویل گرفته شد", terzide: "خیاط ما در حال انجام اصلاح است", hazir: "می‌توانید از فروشگاه تحویل بگیرید", teslim: "لباس به دست شما رسید", kargo: "لباس شما با پست در راه است", kargo_h: "بسته شما در حال آماده‌سازی است" },
    adim_kargo: "ارسال شد", adim_kargo_h: "در حال آماده‌سازی ارسال",
    kd: { hazirlaniyor: "لباس شما برای ارسال آماده می‌شود", yolda: "لباس شما ارسال شد", teslim: "بسته شما تحویل داده شد" },
    kargo: "اطلاعات ارسال", firma: "شرکت پست", takip_no: "کد رهگیری", kargo_tarihi: "تاریخ ارسال", varis: "مقصد",
    takip_et: "رهگیری بسته", kargo_m: "پس از ارسال، کد رهگیری اینجا نمایش داده می‌شود.",
    teslim_yeri: "نشانی تحویل", sube_b: "فروشگاه ما", saatler: "ساعات کاری", ara: "تماس با فروشگاه", yol: "مسیریابی", guncelleme: "آخرین به‌روزرسانی",
  },
  ro: {
    acik: { alindi: "Articolul a fost lăsat în magazinul nostru", terzide: "Croitorul nostru lucrează la ajustare", hazir: "Îl puteți ridica din magazin", teslim: "Articolul a ajuns la dvs.", kargo: "Articolul este pe drum prin curier", kargo_h: "Coletul este în pregătire" },
    adim_kargo: "Expediat", adim_kargo_h: "Se pregătește expedierea",
    kd: { hazirlaniyor: "Articolul se pregătește pentru expediere", yolda: "Articolul a fost expediat", teslim: "Coletul a fost livrat" },
    kargo: "Livrare", firma: "Curier", takip_no: "Număr AWB", kargo_tarihi: "Expediat la", varis: "Destinație",
    takip_et: "Urmărește coletul", kargo_m: "Numărul de urmărire va apărea aici după expediere.",
    teslim_yeri: "Adresă de ridicare", sube_b: "Magazinul nostru", saatler: "Program", ara: "Sunați magazinul", yol: "Indicații", guncelleme: "Ultima actualizare",
  },
  el: {
    acik: { alindi: "Το ρούχο σας παραδόθηκε στο κατάστημά μας", terzide: "Ο ράφτης μας κάνει τη μεταποίηση", hazir: "Μπορείτε να το παραλάβετε από το κατάστημα", teslim: "Το ρούχο σας παραδόθηκε", kargo: "Το ρούχο σας ταξιδεύει με courier", kargo_h: "Το δέμα σας ετοιμάζεται" },
    adim_kargo: "Απεστάλη", adim_kargo_h: "Προετοιμασία αποστολής",
    kd: { hazirlaniyor: "Το ρούχο σας ετοιμάζεται για αποστολή", yolda: "Το ρούχο σας απεστάλη", teslim: "Το δέμα σας παραδόθηκε" },
    kargo: "Αποστολή", firma: "Εταιρεία courier", takip_no: "Αριθμός αποστολής", kargo_tarihi: "Ημ. αποστολής", varis: "Προορισμός",
    takip_et: "Εντοπισμός δέματος", kargo_m: "Ο αριθμός αποστολής θα εμφανιστεί εδώ μόλις σταλεί το δέμα.",
    teslim_yeri: "Διεύθυνση παραλαβής", sube_b: "Το κατάστημά μας", saatler: "Ωράριο", ara: "Κλήση καταστήματος", yol: "Οδηγίες", guncelleme: "Τελευταία ενημέρωση",
  },
  bg: {
    acik: { alindi: "Дрехата е оставена в нашия магазин", terzide: "Нашият шивач прави корекцията", hazir: "Можете да я вземете от магазина", teslim: "Дрехата е при вас", kargo: "Дрехата пътува с куриер", kargo_h: "Пратката се подготвя" },
    adim_kargo: "Изпратено", adim_kargo_h: "Подготвя се за изпращане",
    kd: { hazirlaniyor: "Дрехата се подготвя за изпращане", yolda: "Дрехата е изпратена", teslim: "Пратката е доставена" },
    kargo: "Доставка", firma: "Куриер", takip_no: "Номер за проследяване", kargo_tarihi: "Изпратено на", varis: "Дестинация",
    takip_et: "Проследи пратката", kargo_m: "Номерът за проследяване ще се появи тук след изпращане.",
    teslim_yeri: "Адрес за получаване", sube_b: "Нашият магазин", saatler: "Работно време", ara: "Обадете се", yol: "Упътване", guncelleme: "Последна актуализация",
  },
};
