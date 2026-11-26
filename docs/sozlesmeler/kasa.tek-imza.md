<!-- Sahip: Yusuf. yazan: [Claude#taa-kasa-KS2 2026-11-26]. Yusuf'un kararı (2026-11-26): mağazanın ikinci imzası gelmezse TAA Çatı'ya raporlanır, kasa işlemi durmaz. -->
# kasa.tek-imza

- Tür: veri türü (rapor)
- Sürüm: @1
- Sunan / tek yazar: **TAA Kasa** (`taakasa`; `crates/taa-kasa/src/cati.rs`).
- Okuyan: TAA Çatı (rapor ucu henüz yok; uç gelince bu sözleşmeyle alır).
- Tetik: gün sonu ikinci imza atılmadan (gerekçeyle) gönderildiğinde, `/api/gonder` sonrasında.
- Kuyruk: `<veri kökü>/kasa/cati/giden/tek-imza-<gün sonu kimliği>.json`; teslim edilen `gonderilen`e taşınır (silinmez).
  Çatı ucu yokken taşıyıcı `CatiUcuYok` yer tutucudur: rapor kuyrukta bekler, **kasa işlemi engellenmez**.

## Alanlar
`{"tur":"kasa.tek-imza@1","surum":1,"sube","tarih","cekmece","gun_sonu_id","imzalayan":{"kisi","kisi_rol","kanal","zaman"},"ikinci_imza_durumu":"ikinci imza atmadı","gerekce","olusturma"}`

- `kisi_rol`: `mudur` | `yonetici` (yalnız yöneticiler imzalar); eski kayıtta `null`.
- `gerekce`: ikinci imza atılmama gerekçesi (hazır şablon ya da serbest metin, 5–300 karakter).
- **Tutar yoktur**; ayrıntı aynı gün sonunun `kasa.gunsonu@1` paketindedir.

## Hata kodları
Kasa tarafında rapor yazılamazsa gönderim yine tamamlanır; `/api/gonder` yanıtında `cati.hata` döner.

## Sürüm kuralı
Değişirse `@2` yeni dosya; eski `@1` en az bir sürüm daha yaşar (§18.3).
