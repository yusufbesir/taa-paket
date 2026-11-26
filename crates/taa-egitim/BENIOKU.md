<!-- Sahip: Yusuf. yazan: [Claude#egitim 2026-11-26] -->
# taa-egitim

TAA Eğitim çekirdeği (EG0; ana plan §4.7, mağaza işleyişi planı EG5). Ağ yok, saat yok (zamanı çağıran verir), ilk sürümde sınav yok.

- `src/katalog.rs`: `Ders` (id, başlık, konu, görev etiketleri, süre sn, sürüm) ve `Katalog`. Kasa eğitimi bir konudur (`KONU_KASA = "kasa-egitimi"`). `suz(konu, gorev)` konuya/göreve göre süzer. Silme ucu yok; `yeni_surum` sürümü kesin artırır.
- `src/rehber.rs`: `RehberOkur` (salt okunur: kişi etkin mi, etkin görev kodları) ve `taa_rehber::model::Veri` için `RehberVeri` bağlayıcısı. Kişi adı/e-postası bu crate'te tutulmaz ve günlüğe yazılmaz; yalnız kişi kimliği.
- `src/egitim.rs`: `Egitim` durumu. Atama (`ata`, `gorevden_ata`, `atamayi_iptal_et`; iptal kayıt bırakır), `konum_kaydet` (kaldığı saniye) ve `izledim`, `devam_noktasi`, kişinin kendi listesi `kisi_dersleri`, yalnız `yonetici == true` ile `ozet`. Dersin yeni sürümü eski izlemeyi geçersiz sayar. Her değişiklik önce günlüğe, sonra belleğe işlenir; değişmeyen olay günlüğe yazılmaz. `Egitim::gunlukten` durumu günlükten yeniden kurar.
- `src/gunluk.rs`: yalnız-ekleme JSONL (`Gunluk::ac/ekle/oku`). Dosya kısaltılmaz; yarıda kalan son satır ayrı satırda bırakılır, okurken bozuk satırlar sayılıp atlanır.
- `src/aralik.rs`: HTTP Range hesaplayıcı (`aralik_coz`: tek aralık, 206/416/tam içerik kararı; `ParcaPlani`: eşit parçalara bölme ve kaldığı yerden devam).

Test: `cargo test -p taa-egitim` (39 test).

## Henüz yok
- Video dosyasına bağ: `Ders` içinde `egitim.video@1` havuz kaydı kimliği/dosya yolu alanı yok (havuz kayıt kimliği biçimi bu göreve verilmedi). Range hesaplayıcı toplam bayt sayısını çağırandan alır.
- Konu listesi serbest kimliktir; konuların sabit sözlüğü (kasa dışında) ve görev etiketlerinin rehberdeki `rol` kayıtlarıyla eşleştirilmesi yok (etiket yalnız rol kodu biçimi denetimine girer; AÇIK-R7 rol kodları da belirsiz).
- Yönetici yetkisi: `ozet` yalnız çağıranın verdiği `yonetici` bayrağına bakar; bayrağın kim için doğru sayılacağı (Çatı/RH11 rolleri) bu crate'te çözülmedi. Atama yetkisi de çağıran katmanda denetlenmeli (burada yalnız atayanın rehberde etkin olması aranır).
- "İzledim" elle işarettir; belirli bir yüzdeden (örn. %95) otomatik izlendi sayma kuralı verilmediği için yok. Atanmamış dersi izlemek serbest (izleme kaydı tutulur, özette "atanmamış" görünür).
- Etiketsiz ders hiçbir göreve otomatik atanmaz ve görev süzgecine girmez ("herkese açık" anlamı sözleşmede yok).
- Günlük zincirli/imzalı değildir (taa-zincir kullanılmadı; bütünlük mührü gerekirse eklenir). Dosya kilidi yok: aynı günlüğe tek yazar varsayılır. Saklama süresi ve günlük döndürme (rotasyon) kararı yok; izleme günlüğü kişi kimliği içerdiği için saklama süresi hukuki görüşe bağlı olabilir.
- Sunucu tarafı (EG1: akış ve izleme tablosu H6), program kabuğu, çevrimdışı önbellek (EG2), tema/kur kataloğu (EG4) kapsam dışı.
- Kasa eğitimi içeriği (açılış, devir, gün sonu dersleri, vakalar, sınama soruları) yok; içerik Yusuf'tan beklenir (§10 G3). Sınama soruları ilk sürümde yok.
- Üst sınırlar (5000 ders, 16 etiket, 24 saat süre) plan değil, temkinli varsayımdır.
- `Cargo.lock`: yeni crate için cargo kendiliğinden bir `taa-egitim` girdisi ekledi (yeni dış bağımlılık yok).
