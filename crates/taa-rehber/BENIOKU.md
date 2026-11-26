<!-- Sahip: Yusuf. yazan: [Claude#rehber-model 2026-11-26] -->
# taa-rehber

TaaRehber çekirdeği. İki parça:

- `src/lib.rs`: mevcut CSV okuyucu, doğrulayıcı ve hiyerarşi metni (salt okunur).
- `src/model.rs`: `docs/rehber-sozlesmesi.md` §2-§6 alan modeli. `KisiKaydi`, `BirimKaydi`, `GorevlendirmeKaydi`, `RolKaydi` (serde, bilinmeyen alan reddedilir), kayıt doğrulayıcıları ve `Veri::hatalar()` bütünlük denetimi, bellekte `Depo` (sürümlü yazma, `Cakisma`, silme ucu yok), `gorunum()` şube süzgeci, `eposta_maskele()` günlük maskeleme.

Yazma gerçekte sunucudaki `rehber.yaz@1` hizmetindedir; `Depo` o kuralların bellek içi taklididir. `guncelleme` zamanını çağıran verir (saat bağımlılığı yok).

## Henüz yok
- Şube kodu listesi: `sube_kodu` yalnız boş olmama denetiminden geçer; 15 şube kodunun (`docs/sube-sozlesmesi.md` §3) kendisiyle karşılaştırılmaz (liste bu crate'e verilmedi).
- Mağaza müdürü/sorumlusu rol kodları (AÇIK-R7): `Bakis.mudur_rolleri` çağıranın verdiği küme; `docs/rehber-rolleri.toml` henüz okunmuyor.
- Sözleşme §5 madde 4 alanları ("yalnız ad, eposta, gorev") harfiyen uygulandı: müdür satırında `birim_id` ve `rol` de verilmez, bu yüzden arayüz müdürün hangi mağazaya ait olduğunu bu veriden gösteremez. Gerekirse sözleşme ya da Yusuf kararıyla `birim_id` eklenir.
- Boş kapsam (`sube_kodu` yok ya da boş) ve `tum_oku` yok: yalnız `tur=merkez` birimleri ve oradaki görevliler görünür (departman, mağaza ve şube adresleri görünmez). Sözleşme "yalnız merkez" diyor, departmanın dahil olup olmadığı açık yazmıyor; temkinli yorum seçildi.
- Satır kapsamı (`sube_kapsam`, §10 madde 22, `kapsam_zorunlu`) bu modelde yok: türlerin alan tablosunda geçmediği için eklenmedi; görünürlük birim ve görevlendirmeden türetiliyor. Yazma hizmeti katmanında ele alınmalı.
- E-posta benzersizliği etkin kişiler arasında ve büyük/küçük harf duyarsız; `kisi_ekle`/`kisi_yaz` adresi küçük harfe çevirir (sözleşmede yazmıyor).
- 2000 kişi sınırı pasif kişileri de sayar (kimlik yeniden kullanılmadığı için); sözleşme "en çok 2000 kişi" diyor, pasifin sayılıp sayılmayacağı açık değil.
- Etkin görevlendirme: `durum=etkin` ve bugün `baslangic`..`bitis` aralığında. Asıl görev benzersizliği tarih kesişimine göre denetlenir (devir için eski satırın `bitis` doldurulup yeni satır eklenebilir); sözleşme kesişim kuralını açık yazmıyor.
- `vekil_edilen_id` yalnız var olan kişiye başvuru olarak denetlenir; `tur=vekil` dışında kullanımı yasaklanmadı. `gecici`/`vekil` için `bitis` yalnız önerildiğinden zorunlu değil. Pasif kişiye yeni görevlendirme yazılması engellenmedi.
- Pasif kişi saklama süresi ve aydınlatma notu (AÇIK-R5) Yusuf/hukuki görüşte.
- `rehber.belge@1` (El Kitabı) modeli yok (bu görevin kapsamı dışında).
- Önerisi (`veri.oner@1`), denetim izi, dışa aktarma izni, imzalı yerel önbellek (§7, §9) bu crate'te yok.
- Görev metninde "mevcut 8 test" deniyordu; depoda 5 mevcut test vardı, hepsi geçiyor. Yeni modelin 26 testi var.
