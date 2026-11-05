<!-- Sahip: Yusuf. TaaSüre: teslim tarihi önerisi -->
# teslim.oneri

- Tür: yetenek
- Sürüm: @1
- Sunan: taa-takip (`teslim_oner`)
- İsteyenler: fiş yazan program (TaaFiş/Tutaa); yoksa öneri düğmesi gizli, bağımlılık yok
- Özellik anahtarı: `teslim-oneri` (varsayılan açık; Ayarlar'dan kapatılır, kapalıyken fiş elle tarihlenir)

## İstek alanları
tablo (`sure.json`), yeni fişin kalemleri, usta (boş olabilir), bugün (YYYY-AA-GG), defterdeki fişler

## Yanıt alanları
`sonuc = tarih`: tarih, usta, kuyruk_dk, suresiz_kuyruk_isi, yogun, gerekce
`sonuc = tahmin_yok`: neden ("tahmin yok, elle seç")

## Kurallar
- Öneri otomatik uygulanmaz; personel değiştirebilir. Teslim saati fişteki saattir.
- Gömülü tablo boştur: işlem süreleri ve ustanın günlük dakikası ustadan gelene kadar sonuç hep "tahmin yok".
- Kuyruk = ustanın açık (bekliyor/terzihanede/hazır) fişleri; süresi tabloda olmayan iş uyarı olarak sayılır.
- Usta karşılaştırması/puanı yok. Gerçek ortalamaya geçiş (başladı/bitti dokunuşları) sonraki adım.
