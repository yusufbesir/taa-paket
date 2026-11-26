// TAA müşteri takip: v3 ekleri (kargo, şube kartı, adım açıklamaları).
// Şube bilgisi kamuya açık işletme bilgisidir (fiş başlığında basılı olan); kayıtta yalnız şube kodu gelir.
// Kargo firmaları ve takip bağlantıları burada; takip bağlantısı kalıbı doğrulanmamış firmada yalnız ana sayfa açılır.
// Yazan: [Claude Opus 5.5, TerziTakip başlığı, 2026-11-26].

/** Şube kodu → kamuya açık bilgi (ilk tablo: terziademaltun.com/iletisimbilgileri, 2026-11-05).
 *  Güncel tablo mağaza sunucusundan imzalı olarak gelir (POST /api/subeler, günlük site eşitlemesi) ve bunun üstüne yazılır;
 *  bu dosya yalnız ilk/yedek değerdir. Adresi olmayan şubede "açık adres için merkezi arayın" notu gösterilir.
 *  not: kısa Türkçe ek bilgi (ör. Outlet yanında). wa: şube WhatsApp numarası (boşsa merkez numarası). */
export const SUBELER = {
  MRK01: {
    ad: "İstanbul Merkez Mağaza", adres: "Turgut Özal Mahallesi 68. Sokak No:12 E-5 Karayolu Yan Yol Üzeri, Esenyurt / İstanbul",
    tel: "+902126900202", tel_g: "0212 690 02 02", saat: "09:30–21:30", not: "Outlet mağazamız hemen yanında", wa: "",
  },
  PND01: {
    ad: "İstanbul Pendik Mağaza", adres: "Çınardere Mahallesi Gönenli Mehmet Efendi Caddesi E-5 Yanyol Üzeri No:85, Pendik / İstanbul",
    tel: "+902163750002", tel_g: "0216 375 00 02", wa: "",
  },
  UMR01: {
    ad: "İstanbul Ümraniye Mağaza", adres: "Atakent Mahallesi Alemdağ Caddesi No:326/14, Ümraniye / İstanbul",
    tel: "+905455138025", tel_g: "0545 513 80 25", wa: "",
  },
  ANK01: {
    ad: "Ankara Mağaza", adres: "Yeşilova, Ayaş Ankara Yolu Blv. Gökdemir Tower Sitesi No:103/105, Etimesgut / Ankara",
    tel: "+903125122525", tel_g: "0312 512 25 25", wa: "",
  },
  BRS01: {
    ad: "Bursa Mağaza", adres: "Ataevler Mahallesi, Ata Caddesi No:14 A/B, Nilüfer / Bursa",
    tel: "+902245035034", tel_g: "0224 503 50 34", wa: "",
  },
  SKR01: {
    ad: "Sakarya Mağaza", adres: "Semerciler, Saray Bosna Cd No:4/1, Adapazarı / Sakarya",
    tel: "+902645030035", tel_g: "0264 503 00 35", wa: "",
  },
  DYB01: {
    ad: "Diyarbakır Mağaza", adres: "Pozitif-İnfinity, Talaytepe, 4003. Sokak A Blok No:4A/11, Kayapınar / Diyarbakır",
    tel: "+905304461998", tel_g: "0530 446 19 98", wa: "",
  },
  SVS01: {
    ad: "Sivas Mağaza", adres: "Demircilerardı Mahallesi, Atatürk Bulvarı Dış Kapı No:97/C, Merkez / Sivas",
    tel: "+903465021658", tel_g: "0346 502 16 58", wa: "",
  },
  GZT01: {
    ad: "Gaziantep Mağaza", adres: "Eydibaba Mahallesi, Çelik Sokak No:57/F, Şehitkamil / Gaziantep",
    tel: "+905321755310", tel_g: "0532 175 53 10", wa: "",
  },
  URF01: {
    ad: "Şanlıurfa Mağaza", adres: "GAP Bulvarı Canbeyli Rezidans, Atakent, Şanlıurfa - Diyarbakır Yolu, Karaköprü / Şanlıurfa",
    tel: "+905323099863", tel_g: "0532 309 98 63", wa: "",
  },
  VAN01: {
    ad: "Van Mağaza", adres: "Halilağa Mahallesi, No:111C, İpekyolu / Van",
    tel: "+905016666865", tel_g: "0501 666 68 65", wa: "",
  },
  SRT01: {
    ad: "Siirt Mağaza", adres: "Barış Mahallesi Hükümet Bulvarı No:99/18, Merkez / Siirt",
    tel: "+905435917077", tel_g: "0543 591 70 77", wa: "",
  },
  ELZ01: {
    ad: "Elazığ Mağaza", adres: "Cumhuriyet Mahallesi, Malatya Caddesi No:36 B, Elazığ Merkez / Elazığ",
    tel: "+905016563023", tel_g: "0501 656 30 23", wa: "",
  },
  IZM01: {
    ad: "İzmir Mağaza", adres: "Gazi, Akçay Cd. No:160, 35410 Gaziemir / İzmir",
    tel: "+905527759944", tel_g: "0552 775 99 44", wa: "",
  },
  MUS01: {
    ad: "Muş Mağaza", adres: "Kültür Mahallesi, Atatürk Bulvarı No:60/1, Muş Merkez / Muş",
    tel: "+905330867649", tel_g: "0533 086 76 49", wa: "",
  },
};

/** Merkez WhatsApp numarası (ör. "905xxxxxxxxx"). Şubenin wa'sı da bu da boşsa WhatsApp düğmesi hiç görünmez
 *  ve SSS'deki "WhatsApp'tan ekletebilirsiniz" cümlesi WhatsApp'sız haliyle gösterilir.
 *  YAPILACAK: Yusuf'tan WhatsApp birimi numarası alınıp buraya yazılacak (2026-11-26 itibarıyla gelmedi). */
export const WHATSAPP_MERKEZ = "905448557525";

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

// Renk anahtarı → 13 dil (DILLER sırası: tr en ar ru de fr es zh hi fa ro el bg). Anahtarlar taa_fis::RENKLER ile aynı.
export const RENK = {
  siyah: ["Siyah", "Black", "أسود", "Чёрный", "Schwarz", "Noir", "Negro", "黑色", "काला", "مشکی", "Negru", "Μαύρο", "Черен"],
  lacivert: ["Lacivert", "Navy", "كحلي", "Тёмно-синий", "Marineblau", "Bleu marine", "Azul marino", "藏青色", "गहरा नीला", "سرمه‌ای", "Bleumarin", "Μπλε σκούρο", "Тъмносин"],
  antrasit: ["Antrasit", "Charcoal", "فحمي", "Антрацит", "Anthrazit", "Anthracite", "Antracita", "炭灰色", "चारकोल", "ذغالی", "Antracit", "Ανθρακί", "Антрацит"],
  gri: ["Gri", "Grey", "رمادي", "Серый", "Grau", "Gris", "Gris", "灰色", "स्लेटी", "طوسی", "Gri", "Γκρι", "Сив"],
  "acik-gri": ["Açık gri", "Light grey", "رمادي فاتح", "Светло-серый", "Hellgrau", "Gris clair", "Gris claro", "浅灰色", "हल्का स्लेटी", "طوسی روشن", "Gri deschis", "Ανοιχτό γκρι", "Светлосив"],
  fume: ["Füme", "Smoke grey", "رمادي دخاني", "Дымчатый", "Rauchgrau", "Gris fumé", "Gris humo", "烟灰色", "धूसर", "دودی", "Gri fumuriu", "Φιμέ", "Опушено сив"],
  bej: ["Bej", "Beige", "بيج", "Бежевый", "Beige", "Beige", "Beis", "米色", "बेज", "بژ", "Bej", "Μπεζ", "Бежов"],
  krem: ["Krem", "Cream", "كريمي", "Кремовый", "Creme", "Crème", "Crema", "奶油色", "क्रीम", "کرم", "Crem", "Κρεμ", "Кремав"],
  haki: ["Haki", "Khaki", "كاكي", "Хаки", "Khaki", "Kaki", "Caqui", "卡其色", "खाकी", "خاکی", "Kaki", "Χακί", "Каки"],
  kahverengi: ["Kahverengi", "Brown", "بني", "Коричневый", "Braun", "Marron", "Marrón", "棕色", "भूरा", "قهوه‌ای", "Maro", "Καφέ", "Кафяв"],
  taba: ["Taba", "Tan", "بني فاتح", "Рыжевато-коричневый", "Hellbraun", "Fauve", "Tostado", "棕褐色", "हल्का भूरा", "قهوه‌ای روشن", "Coniac", "Ταμπά", "Светлокафяв"],
  mavi: ["Mavi", "Blue", "أزرق", "Синий", "Blau", "Bleu", "Azul", "蓝色", "नीला", "آبی", "Albastru", "Μπλε", "Син"],
  "acik-mavi": ["Açık mavi", "Light blue", "أزرق فاتح", "Голубой", "Hellblau", "Bleu clair", "Azul claro", "浅蓝色", "हल्का नीला", "آبی روشن", "Albastru deschis", "Γαλάζιο", "Светлосин"],
  beyaz: ["Beyaz", "White", "أبيض", "Белый", "Weiß", "Blanc", "Blanco", "白色", "सफ़ेद", "سفید", "Alb", "Λευκό", "Бял"],
  bordo: ["Bordo", "Burgundy", "عنابي", "Бордовый", "Bordeaux", "Bordeaux", "Burdeos", "酒红色", "मैरून", "زرشکی", "Bordo", "Μπορντό", "Бордо"],
  yesil: ["Yeşil", "Green", "أخضر", "Зелёный", "Grün", "Vert", "Verde", "绿色", "हरा", "سبز", "Verde", "Πράσινο", "Зелен"],
  mor: ["Mor", "Purple", "بنفسجي", "Фиолетовый", "Lila", "Violet", "Morado", "紫色", "बैंगनी", "بنفش", "Mov", "Μωβ", "Лилав"],
  pembe: ["Pembe", "Pink", "وردي", "Розовый", "Rosa", "Rose", "Rosa", "粉色", "गुलाबी", "صورتی", "Roz", "Ροζ", "Розов"],
  kirmizi: ["Kırmızı", "Red", "أحمر", "Красный", "Rot", "Rouge", "Rojo", "红色", "लाल", "قرمز", "Roșu", "Κόκκινο", "Червен"],
  sari: ["Sarı", "Yellow", "أصفر", "Жёлтый", "Gelb", "Jaune", "Amarillo", "黄色", "पीला", "زرد", "Galben", "Κίτρινο", "Жълт"],
  turuncu: ["Turuncu", "Orange", "برتقالي", "Оранжевый", "Orange", "Orange", "Naranja", "橙色", "नारंगी", "نارنجی", "Portocaliu", "Πορτοκαλί", "Оранжев"],
  desenli: ["Desenli", "Patterned", "منقوش", "С узором", "Gemustert", "À motifs", "Estampado", "带图案", "डिज़ाइनदार", "طرح‌دار", "Cu model", "Με σχέδιο", "На шарки"],
};

// Sekmeler, şubeler, WhatsApp ve SSS metinleri. wa_metin içindeki {kod} takip koduyla değişir.
// sss[3] (ek işlem) WhatsApp'ı anar; WhatsApp numarası yokken yanıtı sss_wa_yok ile değişir (takip.mjs, SSS_WA_SIRA).
// bolumler: sekme çubuğunun ekran okuyucu etiketi; kalem_yok: İşlemler boş durumu; adres_yok: adresi girilmemiş şube notu.
export const EK2 = {
  tr: {
    sekme: { ozet: "Özet", islemler: "İşlemler", subeler: "Şubeler", sss: "SSS" },
    burada: "Ürününüz burada", diger_subeler: "Diğer şubelerimiz", wa: "WhatsApp'tan yazın",
    ara_b: "Şube ara", ara_ph: "İl, ilçe ya da şube adı", ara_d: "Ara", ara_yok: "Aramanıza uyan şube bulunamadı.", ara_temizle: "Aramayı temizle",
    wa_metin: "Merhaba, tadilat takip kodum: {kod}. Bilgi almak istiyorum.",
    bagli_m: "Bu fiş {n} fişle birlikte teslim edilecek.", sss_b: "Sıkça sorulan sorular", sss_dugme: "Sıkça sorulan sorular",
    sss_wa_yok: "Evet, ürün teslim edilmeden önce şubemizden ekletebilirsiniz.", bolumler: "Bölümler", kalem_yok: "Bu fiş için işlem ayrıntısı yok.", adres_yok: "Açık adres için merkez mağazamızı arayın:",
    sss: [
      ["Ürünüm ne zaman hazır olur?", "Teslim günü bu sayfada yazar. Ürününüz hazır olunca durum \"Hazır\" olur; saat 17:00'den itibaren alabilirsiniz."],
      ["Hazır ürün ne kadar bekletilir?", "Hazır ürünlerinizi 14 gün içinde alın; 14 gün içinde alınmayan ürünlerden mesul değiliz."],
      ["Fişimi kaybettim ya da ürünü başkası alacak, ne yapmalıyım?", "Şubemize gelin. Telefon numaranızla doğrulama yapılır ve teslim alan kişinin imzası alınır."],
      ["Ek işlem (ütü vb.) ekletebilir miyim?", "Evet, ürün teslim edilmeden önce şubemizden ya da WhatsApp'tan ekletebilirsiniz."],
      ["Ürünüm kargoyla gönderilecekse nasıl takip ederim?", "Kargoya verildiğinde kargo firması ve takip numarası bu sayfada görünür."],
      ["Kişisel verilerim bu sayfada görünür mü?", "Hayır. Bu sayfada adınız, telefonunuz, adresiniz ve ödeme bilgisi yer almaz."],
    ],
  },
  en: {
    sekme: { ozet: "Summary", islemler: "Alterations", subeler: "Stores", sss: "FAQ" },
    burada: "Your item is here", diger_subeler: "Our other stores", wa: "Message us on WhatsApp",
    ara_b: "Search stores", ara_ph: "City, district or store name", ara_d: "Search", ara_yok: "No store matches your search.", ara_temizle: "Clear search",
    wa_metin: "Hello, my alteration tracking code is {kod}. I would like some information.",
    bagli_m: "This receipt will be handed over together with {n} other receipt(s).", sss_b: "Frequently asked questions", sss_dugme: "FAQ",
    sss_wa_yok: "Yes, before handover you can add it at our store.", bolumler: "Sections", kalem_yok: "No alteration details for this receipt.", adres_yok: "For the full address, call our main store:",
    sss: [
      ["When will my item be ready?", "The ready date is shown on this page. When your item is ready the status changes to \"Ready\"; you can collect it from 17:00."],
      ["How long is a ready item kept?", "Please collect ready items within 14 days; we are not responsible for items not collected within 14 days."],
      ["I lost my receipt or someone else will collect it. What should I do?", "Come to our store. We verify with your phone number and the person collecting signs on handover."],
      ["Can I add another service (pressing etc.)?", "Yes, before handover you can add it at our store or via WhatsApp."],
      ["How do I track my item if it is shipped?", "Once shipped, the courier and tracking number appear on this page."],
      ["Is my personal data shown on this page?", "No. Your name, phone, address and payment details are not shown here."],
    ],
  },
  ar: {
    sekme: { ozet: "الملخص", islemler: "التعديلات", subeler: "الفروع", sss: "الأسئلة" },
    burada: "قطعتك هنا", diger_subeler: "فروعنا الأخرى", wa: "راسلنا على واتساب",
    ara_b: "ابحث عن فرع", ara_ph: "المدينة أو الحي أو اسم الفرع", ara_d: "بحث", ara_yok: "لا يوجد فرع يطابق بحثك.", ara_temizle: "مسح البحث",
    wa_metin: "مرحبًا، رمز تتبع التعديل الخاص بي: {kod}. أود الاستفسار.",
    bagli_m: "سيتم تسليم هذا الإيصال مع {n} إيصال آخر.", sss_b: "الأسئلة الشائعة", sss_dugme: "الأسئلة الشائعة",
    sss_wa_yok: "نعم، قبل التسليم من الفرع.", bolumler: "الأقسام", kalem_yok: "لا توجد تفاصيل تعديل لهذا الإيصال.", adres_yok: "للحصول على العنوان الكامل، اتصل بمتجرنا الرئيسي:",
    sss: [
      ["متى تكون قطعتي جاهزة؟", "موعد الاستلام مكتوب في هذه الصفحة. عندما تجهز تتغير الحالة إلى \"جاهزة\"، ويمكنك استلامها من الساعة 17:00."],
      ["كم يتم الاحتفاظ بالقطعة الجاهزة؟", "يرجى الاستلام خلال 14 يومًا؛ لسنا مسؤولين عن القطع التي لا تُستلم خلال 14 يومًا."],
      ["فقدت الإيصال أو سيستلمها شخص آخر، ماذا أفعل؟", "تفضل إلى فرعنا. نتحقق برقم هاتفك ويوقّع المستلم عند التسليم."],
      ["هل يمكنني إضافة خدمة (كي وغيره)؟", "نعم، قبل التسليم من الفرع أو عبر واتساب."],
      ["كيف أتتبع القطعة إذا أُرسلت بالشحن؟", "عند الشحن تظهر شركة الشحن ورقم التتبع في هذه الصفحة."],
      ["هل تظهر بياناتي الشخصية هنا؟", "لا. لا يظهر اسمك أو هاتفك أو عنوانك أو بيانات الدفع."],
    ],
  },
  ru: {
    sekme: { ozet: "Обзор", islemler: "Работы", subeler: "Магазины", sss: "Вопросы" },
    burada: "Ваше изделие здесь", diger_subeler: "Другие наши магазины", wa: "Написать в WhatsApp",
    ara_b: "Поиск магазина", ara_ph: "Город, район или название", ara_d: "Найти", ara_yok: "Магазинов по вашему запросу не найдено.", ara_temizle: "Сбросить поиск",
    wa_metin: "Здравствуйте, мой код отслеживания: {kod}. Хочу уточнить информацию.",
    bagli_m: "Эта квитанция выдаётся вместе с ещё {n}.", sss_b: "Частые вопросы", sss_dugme: "Вопросы",
    sss_wa_yok: "Да, до выдачи — в магазине.", bolumler: "Разделы", kalem_yok: "Подробностей работ по этой квитанции нет.", adres_yok: "Чтобы узнать адрес, позвоните в главный магазин:",
    sss: [
      ["Когда изделие будет готово?", "Дата готовности указана на этой странице. Когда изделие готово, статус меняется на «Готово»; забрать можно с 17:00."],
      ["Сколько хранится готовое изделие?", "Заберите его в течение 14 дней; за изделия, не забранные в течение 14 дней, мы не отвечаем."],
      ["Я потерял квитанцию или заберёт другой человек. Что делать?", "Приходите в магазин. Мы проверим по номеру телефона, получатель расписывается при выдаче."],
      ["Можно добавить услугу (глажка и т. п.)?", "Да, до выдачи — в магазине или через WhatsApp."],
      ["Как отследить отправку курьером?", "После отправки здесь появятся служба доставки и трек-номер."],
      ["Видны ли здесь мои личные данные?", "Нет. Имя, телефон, адрес и оплата здесь не отображаются."],
    ],
  },
  de: {
    sekme: { ozet: "Übersicht", islemler: "Änderungen", subeler: "Geschäfte", sss: "FAQ" },
    burada: "Ihr Artikel ist hier", diger_subeler: "Unsere weiteren Geschäfte", wa: "Per WhatsApp schreiben",
    ara_b: "Geschäft suchen", ara_ph: "Stadt, Bezirk oder Name", ara_d: "Suchen", ara_yok: "Kein Geschäft entspricht Ihrer Suche.", ara_temizle: "Suche zurücksetzen",
    wa_metin: "Hallo, mein Änderungs-Code lautet {kod}. Ich hätte gern eine Auskunft.",
    bagli_m: "Dieser Beleg wird zusammen mit {n} weiteren ausgegeben.", sss_b: "Häufige Fragen", sss_dugme: "FAQ",
    sss_wa_yok: "Ja, vor der Abholung im Geschäft.", bolumler: "Bereiche", kalem_yok: "Für diesen Beleg liegen keine Einzelheiten vor.", adres_yok: "Für die genaue Adresse rufen Sie unser Hauptgeschäft an:",
    sss: [
      ["Wann ist mein Artikel fertig?", "Das Datum steht auf dieser Seite. Ist der Artikel fertig, wechselt der Status auf „Fertig“; Abholung ab 17:00."],
      ["Wie lange wird ein fertiger Artikel aufbewahrt?", "Bitte innerhalb von 14 Tagen abholen; für danach nicht abgeholte Artikel haften wir nicht."],
      ["Beleg verloren oder jemand anderes holt ab?", "Kommen Sie ins Geschäft. Wir prüfen per Telefonnummer, der Abholende unterschreibt."],
      ["Kann ich eine Leistung (Bügeln usw.) hinzufügen?", "Ja, vor der Abholung im Geschäft oder per WhatsApp."],
      ["Wie verfolge ich den Versand?", "Nach dem Versand erscheinen Paketdienst und Sendungsnummer hier."],
      ["Sind meine persönlichen Daten hier sichtbar?", "Nein. Name, Telefon, Adresse und Zahlung werden hier nicht angezeigt."],
    ],
  },
  fr: {
    sekme: { ozet: "Résumé", islemler: "Retouches", subeler: "Boutiques", sss: "FAQ" },
    burada: "Votre article est ici", diger_subeler: "Nos autres boutiques", wa: "Écrire sur WhatsApp",
    ara_b: "Rechercher une boutique", ara_ph: "Ville, quartier ou nom", ara_d: "Rechercher", ara_yok: "Aucune boutique ne correspond à votre recherche.", ara_temizle: "Effacer la recherche",
    wa_metin: "Bonjour, mon code de suivi de retouche est {kod}. J’aimerais un renseignement.",
    bagli_m: "Ce ticket sera remis avec {n} autre(s).", sss_b: "Questions fréquentes", sss_dugme: "FAQ",
    sss_wa_yok: "Oui, avant le retrait, en boutique.", bolumler: "Rubriques", kalem_yok: "Aucun détail de retouche pour ce reçu.", adres_yok: "Pour l'adresse exacte, appelez notre boutique principale :",
    sss: [
      ["Quand mon article sera-t-il prêt ?", "La date figure sur cette page. Quand il est prêt, le statut passe à « Prêt » ; retrait à partir de 17:00."],
      ["Combien de temps un article prêt est-il gardé ?", "Retirez-le sous 14 jours ; nous ne sommes pas responsables des articles non retirés dans ce délai."],
      ["J’ai perdu mon ticket ou quelqu’un d’autre vient le chercher ?", "Venez en boutique. Vérification par votre numéro de téléphone et signature de la personne qui retire."],
      ["Puis-je ajouter un service (repassage, etc.) ?", "Oui, avant le retrait, en boutique ou par WhatsApp."],
      ["Comment suivre un envoi ?", "Une fois expédié, le transporteur et le numéro de suivi apparaissent ici."],
      ["Mes données personnelles sont-elles visibles ici ?", "Non. Nom, téléphone, adresse et paiement n’apparaissent pas."],
    ],
  },
  es: {
    sekme: { ozet: "Resumen", islemler: "Arreglos", subeler: "Tiendas", sss: "Preguntas" },
    burada: "Su prenda está aquí", diger_subeler: "Nuestras otras tiendas", wa: "Escríbanos por WhatsApp",
    ara_b: "Buscar tienda", ara_ph: "Ciudad, distrito o nombre", ara_d: "Buscar", ara_yok: "Ninguna tienda coincide con su búsqueda.", ara_temizle: "Borrar búsqueda",
    wa_metin: "Hola, mi código de seguimiento es {kod}. Quisiera información.",
    bagli_m: "Este recibo se entregará junto con {n} más.", sss_b: "Preguntas frecuentes", sss_dugme: "Preguntas",
    sss_wa_yok: "Sí, antes de la entrega, en tienda.", bolumler: "Secciones", kalem_yok: "No hay detalles de arreglos para este recibo.", adres_yok: "Para la dirección completa, llame a nuestra tienda principal:",
    sss: [
      ["¿Cuándo estará lista mi prenda?", "La fecha aparece en esta página. Cuando esté lista, el estado cambia a «Lista»; puede recogerla desde las 17:00."],
      ["¿Cuánto tiempo se guarda una prenda lista?", "Recójala en 14 días; no nos responsabilizamos de prendas no recogidas en ese plazo."],
      ["Perdí el recibo o la recogerá otra persona, ¿qué hago?", "Venga a la tienda. Verificamos con su teléfono y quien recoge firma la entrega."],
      ["¿Puedo añadir un servicio (planchado, etc.)?", "Sí, antes de la entrega, en tienda o por WhatsApp."],
      ["¿Cómo sigo un envío?", "Al enviarse, la mensajería y el número de seguimiento aparecen aquí."],
      ["¿Se ven mis datos personales aquí?", "No. Nombre, teléfono, dirección y pago no se muestran."],
    ],
  },
  zh: {
    sekme: { ozet: "概览", islemler: "修改项目", subeler: "门店", sss: "常见问题" },
    burada: "您的衣物在此", diger_subeler: "其他门店", wa: "通过 WhatsApp 联系",
    ara_b: "搜索门店", ara_ph: "城市、区或门店名称", ara_d: "搜索", ara_yok: "没有符合搜索条件的门店。", ara_temizle: "清除搜索",
    wa_metin: "您好，我的修改跟踪码是 {kod}，想咨询一下。",
    bagli_m: "此单据将与另外 {n} 张单据一起交付。", sss_b: "常见问题", sss_dugme: "常见问题",
    sss_wa_yok: "可以，交付前到店追加。", bolumler: "栏目", kalem_yok: "此收据暂无修改明细。", adres_yok: "如需详细地址，请致电总店：",
    sss: [
      ["衣物什么时候完成？", "完成日期显示在本页。完成后状态变为“已完成”，17:00 起可领取。"],
      ["完成的衣物保留多久？", "请在 14 天内领取；超过 14 天未领取的衣物本店不负责。"],
      ["单据丢失或由他人代领怎么办？", "请到门店，凭您的电话号码核实，领取人当场签字。"],
      ["可以追加服务（熨烫等）吗？", "可以，交付前到店或通过 WhatsApp 追加。"],
      ["快递寄送如何跟踪？", "寄出后本页会显示快递公司和运单号。"],
      ["本页会显示我的个人信息吗？", "不会。本页不显示姓名、电话、地址和付款信息。"],
    ],
  },
  hi: {
    sekme: { ozet: "सारांश", islemler: "बदलाव", subeler: "दुकानें", sss: "सवाल" },
    burada: "आपका कपड़ा यहाँ है", diger_subeler: "हमारी अन्य दुकानें", wa: "WhatsApp पर लिखें",
    ara_b: "दुकान खोजें", ara_ph: "शहर, ज़िला या दुकान का नाम", ara_d: "खोजें", ara_yok: "आपकी खोज से कोई दुकान नहीं मिली।", ara_temizle: "खोज साफ़ करें",
    wa_metin: "नमस्ते, मेरा ट्रैकिंग कोड {kod} है। मुझे जानकारी चाहिए।",
    bagli_m: "यह रसीद {n} अन्य रसीदों के साथ दी जाएगी।", sss_b: "अक्सर पूछे जाने वाले सवाल", sss_dugme: "सवाल",
    sss_wa_yok: "हाँ, देने से पहले दुकान पर।", bolumler: "अनुभाग", kalem_yok: "इस रसीद के लिए कोई विवरण नहीं है।", adres_yok: "पूरा पता जानने के लिए हमारी मुख्य दुकान पर कॉल करें:",
    sss: [
      ["मेरा कपड़ा कब तैयार होगा?", "तारीख इस पेज पर लिखी है। तैयार होने पर स्थिति \"तैयार\" हो जाती है; 17:00 से ले सकते हैं।"],
      ["तैयार कपड़ा कितने दिन रखा जाता है?", "14 दिन के भीतर ले जाएँ; उसके बाद न लिए गए कपड़ों की ज़िम्मेदारी हमारी नहीं।"],
      ["रसीद खो गई या कोई और लेने आएगा?", "दुकान पर आएँ। आपके फ़ोन नंबर से पुष्टि होगी और लेने वाला हस्ताक्षर करेगा।"],
      ["क्या मैं इस्त्री जैसी सेवा जोड़ सकता हूँ?", "हाँ, देने से पहले दुकान पर या WhatsApp से।"],
      ["कूरियर से भेजा जाए तो कैसे ट्रैक करूँ?", "भेजने के बाद कूरियर और ट्रैकिंग नंबर यहाँ दिखेगा।"],
      ["क्या मेरी निजी जानकारी यहाँ दिखती है?", "नहीं। नाम, फ़ोन, पता और भुगतान यहाँ नहीं दिखते।"],
    ],
  },
  fa: {
    sekme: { ozet: "خلاصه", islemler: "اصلاحات", subeler: "شعبه‌ها", sss: "پرسش‌ها" },
    burada: "لباس شما اینجاست", diger_subeler: "شعبه‌های دیگر ما", wa: "پیام در واتساپ",
    ara_b: "جستجوی شعبه", ara_ph: "شهر، منطقه یا نام شعبه", ara_d: "جستجو", ara_yok: "شعبه‌ای با جستجوی شما پیدا نشد.", ara_temizle: "پاک کردن جستجو",
    wa_metin: "سلام، کد پیگیری اصلاح من {kod} است. می‌خواهم اطلاعات بگیرم.",
    bagli_m: "این رسید همراه با {n} رسید دیگر تحویل داده می‌شود.", sss_b: "پرسش‌های پرتکرار", sss_dugme: "پرسش‌ها",
    sss_wa_yok: "بله، پیش از تحویل در شعبه.", bolumler: "بخش‌ها", kalem_yok: "برای این رسید جزئیات تعمیر وجود ندارد.", adres_yok: "برای نشانی کامل با فروشگاه مرکزی تماس بگیرید:",
    sss: [
      ["لباسم کی آماده می‌شود؟", "تاریخ در همین صفحه آمده است. پس از آماده شدن وضعیت «آماده» می‌شود و از ساعت 17:00 قابل تحویل است."],
      ["لباس آماده تا کی نگه داشته می‌شود؟", "لطفاً ظرف 14 روز تحویل بگیرید؛ پس از آن مسئولیتی نداریم."],
      ["رسید را گم کرده‌ام یا کس دیگری تحویل می‌گیرد؟", "به شعبه بیایید. با شماره تلفن شما تأیید می‌شود و تحویل‌گیرنده امضا می‌کند."],
      ["می‌توانم خدمتی (اتو و...) اضافه کنم؟", "بله، پیش از تحویل در شعبه یا از طریق واتساپ."],
      ["اگر با پست ارسال شود چگونه پیگیری کنم؟", "پس از ارسال، شرکت پست و کد رهگیری اینجا نمایش داده می‌شود."],
      ["آیا اطلاعات شخصی من اینجا دیده می‌شود؟", "خیر. نام، تلفن، نشانی و پرداخت نمایش داده نمی‌شود."],
    ],
  },
  ro: {
    sekme: { ozet: "Rezumat", islemler: "Ajustări", subeler: "Magazine", sss: "Întrebări" },
    burada: "Articolul dvs. este aici", diger_subeler: "Celelalte magazine", wa: "Scrieți-ne pe WhatsApp",
    ara_b: "Caută magazin", ara_ph: "Oraș, cartier sau nume", ara_d: "Caută", ara_yok: "Niciun magazin nu corespunde căutării.", ara_temizle: "Șterge căutarea",
    wa_metin: "Bună ziua, codul meu de urmărire este {kod}. Aș dori informații.",
    bagli_m: "Acest bon se predă împreună cu încă {n}.", sss_b: "Întrebări frecvente", sss_dugme: "Întrebări",
    sss_wa_yok: "Da, înainte de predare, la magazin.", bolumler: "Secțiuni", kalem_yok: "Nu există detalii de retușuri pentru acest bon.", adres_yok: "Pentru adresa completă, sunați la magazinul central:",
    sss: [
      ["Când va fi gata articolul?", "Data apare pe această pagină. Când e gata, starea devine „Gata”; îl puteți ridica de la 17:00."],
      ["Cât timp este păstrat un articol gata?", "Ridicați-l în 14 zile; nu răspundem pentru articolele neridicate în acest termen."],
      ["Am pierdut bonul sau îl ridică altcineva?", "Veniți la magazin. Verificăm cu numărul de telefon, iar persoana care ridică semnează."],
      ["Pot adăuga un serviciu (călcat etc.)?", "Da, înainte de predare, la magazin sau pe WhatsApp."],
      ["Cum urmăresc un colet?", "După expediere, curierul și numărul AWB apar aici."],
      ["Datele mele personale apar aici?", "Nu. Numele, telefonul, adresa și plata nu sunt afișate."],
    ],
  },
  el: {
    sekme: { ozet: "Σύνοψη", islemler: "Μεταποιήσεις", subeler: "Καταστήματα", sss: "Ερωτήσεις" },
    burada: "Το ρούχο σας είναι εδώ", diger_subeler: "Άλλα καταστήματά μας", wa: "Γράψτε μας στο WhatsApp",
    ara_b: "Αναζήτηση καταστήματος", ara_ph: "Πόλη, περιοχή ή όνομα", ara_d: "Αναζήτηση", ara_yok: "Κανένα κατάστημα δεν ταιριάζει.", ara_temizle: "Καθαρισμός αναζήτησης",
    wa_metin: "Γεια σας, ο κωδικός παρακολούθησής μου είναι {kod}. Θα ήθελα πληροφορίες.",
    bagli_m: "Αυτή η απόδειξη παραδίδεται μαζί με άλλες {n}.", sss_b: "Συχνές ερωτήσεις", sss_dugme: "Ερωτήσεις",
    sss_wa_yok: "Ναι, πριν την παράδοση, στο κατάστημα.", bolumler: "Ενότητες", kalem_yok: "Δεν υπάρχουν λεπτομέρειες μεταποίησης για αυτή την απόδειξη.", adres_yok: "Για την πλήρη διεύθυνση, καλέστε το κεντρικό κατάστημα:",
    sss: [
      ["Πότε θα είναι έτοιμο;", "Η ημερομηνία φαίνεται σε αυτή τη σελίδα. Όταν είναι έτοιμο, η κατάσταση γίνεται «Έτοιμο»· παραλαβή από τις 17:00."],
      ["Πόσο φυλάσσεται ένα έτοιμο ρούχο;", "Παραλάβετε εντός 14 ημερών· δεν ευθυνόμαστε για ρούχα που δεν παραλήφθηκαν."],
      ["Έχασα την απόδειξη ή θα το παραλάβει άλλος;", "Ελάτε στο κατάστημα. Επαλήθευση με το τηλέφωνό σας και υπογραφή του παραλήπτη."],
      ["Μπορώ να προσθέσω υπηρεσία (σιδέρωμα κ.λπ.);", "Ναι, πριν την παράδοση, στο κατάστημα ή μέσω WhatsApp."],
      ["Πώς παρακολουθώ μια αποστολή;", "Μετά την αποστολή, η εταιρεία courier και ο αριθμός εμφανίζονται εδώ."],
      ["Φαίνονται εδώ τα προσωπικά μου στοιχεία;", "Όχι. Όνομα, τηλέφωνο, διεύθυνση και πληρωμή δεν εμφανίζονται."],
    ],
  },
  bg: {
    sekme: { ozet: "Обобщение", islemler: "Корекции", subeler: "Магазини", sss: "Въпроси" },
    burada: "Дрехата ви е тук", diger_subeler: "Другите ни магазини", wa: "Пишете ни в WhatsApp",
    ara_b: "Търсене на магазин", ara_ph: "Град, район или име", ara_d: "Търси", ara_yok: "Няма магазин, отговарящ на търсенето.", ara_temizle: "Изчисти търсенето",
    wa_metin: "Здравейте, кодът ми за проследяване е {kod}. Искам информация.",
    bagli_m: "Тази разписка се предава заедно с още {n}.", sss_b: "Често задавани въпроси", sss_dugme: "Въпроси",
    sss_wa_yok: "Да, преди предаването, в магазина.", bolumler: "Раздели", kalem_yok: "Няма подробности за поправки по тази бележка.", adres_yok: "За точния адрес се обадете в централния магазин:",
    sss: [
      ["Кога ще е готова дрехата?", "Датата е на тази страница. Когато е готова, статусът става „Готово“; може да я вземете след 17:00."],
      ["Колко време се пази готовата дреха?", "Вземете я до 14 дни; не носим отговорност за невзети след този срок."],
      ["Изгубих разписката или друг ще я вземе?", "Елате в магазина. Проверяваме по телефонния ви номер, получателят се подписва."],
      ["Мога ли да добавя услуга (гладене и др.)?", "Да, преди предаването, в магазина или по WhatsApp."],
      ["Как да проследя пратка?", "След изпращане тук се показват куриерът и номерът за проследяване."],
      ["Виждат ли се личните ми данни тук?", "Не. Име, телефон, адрес и плащане не се показват."],
    ],
  },
};
