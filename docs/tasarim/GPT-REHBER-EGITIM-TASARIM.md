yazan: [GPT gpt-6.2-sol]
Oturum: [Codex#20261126-0536]
Sahip: Yusuf / TAA
Açıklama: TAA Rehber ve TAA Eğitim için masaüstü arayüz tasarımı. Tasarımın kapsamı Yusuf'un kararıyla, bu dosyanın içeriği dosyayı yazan Codex tarafından güncellenir.

# TAA Rehber ve Eğitim

## Özet

İki uygulama aynı üst çubuğu, sol gezinmeyi ve hesap menüsünü kullanır. Rehber, kişi ve görev bulmayı, el kitabına erişmeyi ve anonim bildirim yapmayı sağlar. Eğitim, konuya veya göreve göre video bulmayı, kaldığı yerden devam etmeyi ve atamaları takip etmeyi sağlar. Yönetici girişi, rehber kayıtlarını düzenlemeye yönelik ekleme işlemlerini ve eğitim yönetimini açar.

## Tasarım dili

- **Mezura.** Açılışta kısa ölçü çizgileri bir hat üzerinde belirir ve yerini giriş kartına bırakır. Aynı çizgi yükleme ekranında ve oynatıcı zaman çizelgesinde tekrar eder. Uygulama kabuğunda hareket, durum değişikliğini göstermek için kullanılır.
- **Sade yüzeyler.** Bir ekranda bir baskın işlem, kısa etiketler, okunaklı satırlar ve dengeli boşluk. Başlıklar, gruplar ve seçili öğeler boyut, ağırlık ve biçimle ayrılır.
- **Modüler yerleşim.** Arama ve filtreler, ağaç, liste, ayrıntı paneli, formlar ve oynatıcı bağımsız bileşenlerdir. 1280 px genişliğinde 232 px sol gezinme ve 240 px rehber ağacı kullanılır. İçerik daraldığında ayrıntı alanı listenin altına iner.
- **Kabuk.** 60 px üst çubuk aktif temanın rengini, sol gezinti aktif bölümün işaretini alır. Hesap düğmesi sol alta sabitlenir. Menü sırası **kullanıcı → çizgi → Yönetici girişi → çizgi → Ayarlar / tema**. Yönetici oturumunda aynı sıradaki giriş satırı yönetici çıkışına dönüşür.

## Ekran akışı

1. **Açılış → Giriş → Yükleniyor → Kişiler ve Görevler.** Açılış otomatik ilerler. Masaüstü iskeletinde tek düğmeyle örnek oturum başlatılır. Daha önce seçilen tema açılıştan itibaren uygulanır.
2. **Kişiler ve Görevler → Ayrıntı → Adresi kopyala.** Arama, şube ve görev süzgeçleri birlikte uygulanır. Hiyerarşi düğümleri açılıp kapanır. Birim seçimi listeyi sınırlar. Satıra tıklamak ayrıntıyı aynı ekranda açar.
3. **El Kitabı → Bölüm → Madde.** İçindekilerden seçilen bölüm sağdaki okuma alanına gelir.
4. **İhbar Hattı → Kategori ve açıklama → Gönder → Alındı.** Anonim seçim açık başlar. İletişim isteğe bağlıdır ve anonim seçim kapatılınca görünür. Geçerli form tek gönderim düğmesiyle tamamlanır.
5. **Eğitim → Video → Oynat.** Konu ve görev süzgeçleri birlikte uygulanır. Varsa son yarım kalan eğitim için devam düğmesi gösterilir. Atamalar listede etiketlenir.
6. **Hesap → Yönetici girişi → Rehber veya Eğitim.** Rehberde kişi, birim ve görevlendirme formları açılır. Eğitimde video atama ve izleme özeti erişilebilir olur. Yönetici çıkışı açık yönetim ekranını kapatır ve eğitim listesine veya önceki rehber ekranına döndürür.
7. **Hesap → Ayarlar / tema.** Açık, Koyu veya Tek renkli seçim anında uygulanır ve cihazda hatırlanır.

## Ekran yerleşimleri

### Açılış, giriş ve yükleniyor

```text
  ACILIS                  GIRIS                    YUKLENIYOR
+----------------+      +----------------+      +----------------+
|      TAA       |      |      TAA       |      |      TAA       |
|  | || | || |   | ---> | Rehber / Egitim| ---> |  | || | || |   |
|     Mezura     |      | [Oturumu ac]   |      | Hazirlaniyor   |
+----------------+      +----------------+      +----------------+
```

### Kişiler ve Görevler

```text
+ TAA / Rehber -----------------------------------------------------------+
| KISILER VE GOREVLER     [Kisi / gorev ara] [Sube v] [Gorev v]            |
| Kisiler ve Gorevler  | Hiyerarsi       | Sonuclar        | Kisi         |
| El Kitabi            | v Genel Merkez  | Ornek Kisi 1 >   | Birim        |
| Ihbar Hatti          |   > Idari Birim | Ornek Kisi 2 >   | Gorev        |
| Egitim               | > Sube 1        |                 | Adres        |
|                      | [Tum birimler]  |                 | [Kopyala]    |
| [Hesap v]            | Yonetici: [+ Kisi] [+ Birim] [+ Gorevlendirme]   |
+-------------------------------------------------------------------------+
```

### El Kitabı ve İhbar Hattı

```text
+ TAA / Rehber -----------------------------------------------+
| Sol gezinme | EL KITABI                                     |
|             | Icindekiler    | Bolum basligi               |
|             | > Baslangic    | Ozet ve maddeler            |
| [Hesap v]   | > Is akislari  |                              |
+-------------------------------------------------------------+

+ TAA / Rehber -----------------------------------------------+
| Sol gezinme | IHBAR HATTI                                   |
|             | [Kategori v]                                  |
|             | [Aciklama..............................]     |
|             | [x] Anonim gonder                             |
|             | [Iletisim (istege bagli, anonimlik kapaliyken)]|
| [Hesap v]   | [Gonder]        Alindi / Hata                  |
+-------------------------------------------------------------+
```

### Eğitim listesi, oynatıcı ve yönetim

```text
+ TAA / Egitim -----------------------------------------------------------+
| Sol gezinme | EGITIM                  [Kaldigin yerden devam et]       |
|             | [Konu v] [Gorev v]                                        |
|             | Video listesi           | Oynatici                       |
|             | > Kasa egitimi   %30    | Baslik ve video alani            |
|             | > Rehber kullanimi      | [Oynat] [Zaman surgusu]          |
| [Hesap v]   | > El kitabi             | Sure / ilerleme                 |
+-------------------------------------------------------------------------+

+ TAA / Egitim -----------------------------------------------+
| Sol gezinme | IZLEME OZETI                  [+ Atama]       |
|             | Kisi          Egitim         Ilerleme        |
|             | Ornek Kisi 1  Kasa egitimi   %30             |
| [Hesap v]   | Ornek Kisi 2  Kasa egitimi   Baslamadi       |
+-------------------------------------------------------------+
```

### Hesap, tema ve yönetim formları

```text
+ Hesap menusu -------+     + Ayarlar / tema -----------------+
| Ornek Kisi 1        |     | (o) Acik ( ) Koyu ( ) Tek renkli|
| ------------------- | --> |                       [Kapat]  |
| Yonetici girisi     |     +---------------------------------+
| ------------------- |
| Ayarlar / tema      |     + Yonetim formu ------------------+
+---------------------+     | Kisi / Birim / Gorevlendirme    |
                            | veya Egitim atamasi             |
                            | [Secili isleme ait alanlar]     |
                            | [Vazgec]             [Kaydet]   |
                            +---------------------------------+
```

## Bileşenler ve davranışlar

| Bileşen | Davranış |
| --- | --- |
| Üst çubuk ve sol gezinme | Etkin uygulama üst çubukta görünür. Sol gezinme ekranı işaretler. Eğitim izleme özeti yönetici oturumunda görünür. |
| Hesap menüsü ve tema seçici | Hesap düğmesi menüyü açar. Tema radyo düğmeleriyle seçilir. Escape veya menü dışına tıklamak menüyü kapatır. |
| Arama, şube ve görev süzgeçleri | Arama kişi adı, görev, şube ve birim üzerinde çalışır. Süzgeçler ve seçili hiyerarşi düğümü birlikte uygulanır. Sonuç kalmazsa süzgeçler temizlenebilir. |
| Hiyerarşi ağacı | Ana birimler açılıp kapanır. Birim seçmek birimi ve alt birimlerini getirir. Tüm birimler düğmesi ağaç seçimini kaldırır. |
| Kişi listesi ve ayrıntı | Satır, mevcut görevleri, birimleri ve kişinin örnek adresini gösteren ayrıntıyı açar. Kopyalama sonrası durum bildirilir. Pano kullanılamazsa adres alanı seçilir. |
| Rehber yönetim formları | Kişi formu ad ve örnek adres alır. Birim formu ad ve bağlı birim, görevlendirme formu kişi, birim ve görev alır. Ekleme sonrası ilgili görünüm güncellenir. |
| El kitabı içindekiler | Bölüm seçildiğinde başlık ve maddeler aynı okuma panelinde değişir. |
| İhbar formu | Kategori ve açıklama zorunludur. Anonim seçim açıldığında iletişim alanı temizlenir. Başarıdan sonra form temizlenir ve anonimlik yeniden açılır. |
| Eğitim listesi ve atama | Konu ve görev seçicileri videoları daraltır. Yönetici bir video ile bir örnek kişiyi eşler. Aynı atama tekrarlanırsa formda açıklama görünür. |
| Oynatıcı | Oynat, duraklat ve zaman çizelgesinden ilerle. Son konum hatırlanır. İzleme yüzdesi oynatılan süreye göre hesaplanır. Kasa eğitiminde örnek slaytlar bulunur. |
| İzleme özeti | Yönetici kişi, eğitim ve izleme durumunu görür. Özet atanan ve izlenen eğitimleri gösterir. |

## Tema belirteçleri

| Belirteç | Açık | Koyu | Tek renkli |
| --- | --- | --- | --- |
| `--bg` / `--surface` / `--subtle` | `#f4f7f6` / `#ffffff` / `#edf3f0` | `#101815` / `#18231f` / `#23302a` | `#f1f1f1` / `#ffffff` / `#e8e8e8` |
| `--text` / `--muted` | `#172620` / `#485e54` | `#edf6ef` / `#bbcdc2` | `#1c1c1c` / `#505050` |
| `--line` | `#cbd9d1` | `#3e5248` | `#c6c6c6` |
| `--accent` / `--on-accent` | `#145c46` / `#ffffff` | `#b4ecd1` / `#102820` | `#292929` / `#ffffff` |
| `--bar` / `--on-bar` | `#174633` / `#ffffff` | `#234839` / `#f0f8f3` | `#282828` / `#ffffff` |
| `--focus` / `--danger` | `#176749` / `#a02834` | `#b4ecd1` / `#ffb4bb` | `#303030` / `#303030` |

- **Aralık** `--s1` 4 px, `--s2` 8 px, `--s3` 12 px, `--s4` 16 px, `--s5` 24 px, `--s6` 32 px. Kart içi boşluk 16 px, ana panel aralığı 16 px, sayfa iç boşluğu 24 px.
- **Yazı** Sistem yazı tipi. `--type-small` 13 px, `--type-body` 16 px, `--type-panel` 20 px, `--type-page` 32 px. Gövde satır yüksekliği 1,5. Süre göstergesinde eşit genişlikli rakamlar.
- **Biçim** Kart köşesi 12 px, düğme köşesi 8 px, ana kontroller 44 px, odak halkası 3 px. Durumlar metin ve biçimle de işaretlenir.

## Boş, hata ve yükleniyor durumları

| Alan | Boş | Hata | Yükleniyor |
| --- | --- | --- | --- |
| Rehber | “Eşleşen kişi yok.” ve “Süzgeçleri temizle”. | “Kişiler yüklenemedi.” ve “Yeniden dene”. Kopyalama hatasında adres alanı seçilir. | Ağaç ve liste yerinde kalır. Liste alanı “Kişiler yükleniyor” durumunu bildirir. |
| El Kitabı | “Bu bölüm henüz boş.” İçindekiler erişilebilir kalır. | “Bölüm açılamadı.” ve “Yeniden dene”. | Okuma alanı “Bölüm yükleniyor” durumunu bildirir. |
| İhbar Hattı | Form boş, anonimlik açık. | Eksik alanlar etiketleriyle gösterilir. İletim hatasında form içeriği korunur ve yeniden gönderilebilir. | “Gönderiliyor” metni gösterilir. İşlem sürerken ikinci gönderim engellenir. |
| Eğitim | “Bu süzgeçlerle eşleşen eğitim yok.” ve “Süzgeçleri temizle”. Yönetici özeti boşsa “Henüz izleme veya atama kaydı yok.” | “Video başlatılamadı.” ve “Yeniden dene”. Son kayıtlı konum korunur. | Oynatıcı alanı boyutunu korur ve “Video hazırlanıyor” durumunu bildirir. |
| Yönetim formları | İlk seçimler açıkça etiketlenir. | Hata ilgili alanda veya form başında gösterilir. Form değerleri korunur. | Kayıt sırasında eylem düğmesi “Kaydediliyor” durumuna geçer. |

## Erişilebilirlik

- Klavyeyle gezinmede mantıksal sıra, görünür odak, ilk bağlantı olarak “İçeriğe geç” ve etkin sayfa için `aria-current` kullanılır. Ağaçta doğal açılır öğeler, listede standart düğmeler kullanılır.
- Dinamik sonuç ve işlem bildirimleri `aria-live="polite"` üzerinden okunur. Form alanları görünür etiketlidir. Yönetim pencerelerinde Escape kapatır ve odak açan denetime döner.
- Normal metin için en az 4,5:1, büyük metin ve arayüz sınırları için 3:1 kontrast hedeflenir. Seçim, atama ve izleme durumu metin veya belirgin bir işaretle de gösterilir.
- Hareket azaltma tercihi etkinse açılış hareketi ve otomatik bekleme atlanır. Oynatıcı kullanıcı eylemiyle başlar. Üretim videolarında Türkçe altyazı ve yazılı döküm sunulur.
- %200 yakınlaştırmada içerik sırası korunur, liste ve ayrıntı alt alta yerleşir. Zaman çizelgesi klavyeyle ilerletilir. Süre ve ilerleme metin olarak da görünür.

## İlham ilkeleri — doğrulanmadı, bilgiden

Aşağıdaki satırlar adları verilen kaynaklarla ilişkilendirilen genel tasarım ilkeleridir. Bu çalışma sırasında siteler açılmadı.

| Kaynak | Tasarıma taşınan ilke |
| --- | --- |
| Mobbin iOS | Tek öne çıkan eylem, seçilen öğeyi işaretleyen gruplu liste, tutarlı denetim konumu. |
| Refero | Başlık, açıklama ve ikincil bilginin okunaklı tipografi hiyerarşisi. |
| Pageflows iOS | Giriş, bekleme ve tamamlanma gibi ara durumların akış içinde görünmesi. |
| SiteInspire | Birbirini izleyen içerik bloklarında dengeli boşluk ve hizalama. |
| Land-book | Vurgu rengini ana eylem ve seçili durumla sınırlayan renk kullanımı. |
| Minimal Gallery | Sade yüzey, ölçülü çerçeve ve kararlı ızgara. |
| Dribbble mobil ve ürün tasarımı | Durum, ilerleme ve filtreleri anlaşılır kılan küçük, tekrar kullanılabilir bileşenler. |

## HTML iskeletinin kapsamı

`rehber-egitim-iskelet.html` tarayıcıda tek başına açılır. Eğitim oynatıcısı yerleşik yazılı slaytları süreye göre ilerletir. Yönetici girişi örnek erişimdir. Rehber eklemeleri ve atamalar açık sekmede tutulur. Tema ve örnek izleme konumu tarayıcı izin verdiğinde yerel olarak saklanır. İhbar gönderimi örnek bir alındı bildirimi üretir ve girilen içeriği saklamaz. Üretimde oturum, yönetici yetkisi, kayıt erişimi ve bildirim iletimi sunucuda uygulanır.

## Doğrulama durumu ([Claude#gozetimsiz-tasarim])
- İçerik Codex (gpt-6.2-sol) çıktısıdır; doğrulanmış sayılmaz. Derleme/test yapılmadı; HTML iskeleti tarayıcıda açılıp görsel olarak denenmedi.
- İlham siteleri açılmadı; ilkeler modelin bilgisinden, "doğrulanmadı". Farklı aileden inceleme ve güvenlik incelemesi uygulamadan önce yapılmalı.
- Codex çalıştı, kota/hata sorunu olmadı. Dış ajan dağıtımı (Gemini vb.) bu işte yapılmadı.
