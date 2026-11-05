<!-- Sahip: Yusuf. yazan: [Claude#taa-paket-T2 2026-11-05] -->
# Zip paket biçimi (T2-A2)

`taa-kur kur <paket.zip>` klasör paketiyle aynı içeriği tek zip olarak kurar. Üretim: `python3 tools/paket_uret.py <crate-klasoru> --zip <cikti.zip> [--bin <exe>]` (stdlib `zipfile`, deflate; var olan zip ezilmez).

## Kurallar
- `paket.json` zipin **kökünde** olmalıdır (alt klasörde değil). Şema `docs/T1-TASARIM.md` §1.2 ile aynıdır.
- Argüman `.zip` ile bitiyorsa ve klasör değilse zip sayılır. Zip `<kök>/gecici-<pid>/` altına açılır, sonra klasör kurulumu (doğrulama, çözücü, kayıt) aynen çalışır; geçici klasör iş bitince (hata dahil) `<kök>/eski/gecici-<pid>-<zaman>` altına taşınır. Silme yoktur.
- Sınırlar: en çok 10.000 girdi; açılmış toplam en çok 2 GiB. Her girdinin bildirilen boyutu önce toplanır, sınır aşılırsa **hiçbir şey açılmadan** reddedilir (çıkış 2). Açarken gerçek bayt sayısı da denetlenir (bildirilenden büyük çıkarsa zip bombası şüphesiyle durulur).
- Yol kuralları `paket::yol_dogrula` ile aynıdır: `..`, mutlak yol, sürücü harfi, `\`, denetim karakteri reddedilir. Sembolik bağ girdileri ve yinelenen adlar reddedilir.
- SHA256 denetimi: açılan klasör `paket.json` ile birebir doğrulanır (bozuk sha reddedilir, kayıt ve depo değişmez).

## `imza` alanı (T3 imza)
`paket.json` içinde `imza` alanı ayrılmıştır; şimdilik `null` yazılır. Rust bu alanı **yok sayar ve doğrulamaz**; imza doğrulaması T3'tedir (ed25519 bağımlılık onayı beklenir). Dolu bir `imza` kurulumu engellemez.

## `taa-kur tani --json` veri alanları
`veri_kok` (`TAA_VERI_KOK`, yoksa Windows `%APPDATA%\TAA\veri`, diğerlerinde `$XDG_DATA_HOME/TAA/veri` ya da `~/.local/share/TAA/veri`) ve `veri_alanlari`: kökteki her `<alan>/SAHIP.toml` için `{alan, sahip, gecici, sahip_kurulu}`. `sahip_kurulu`, `sahip` adının `kayit.json` paketler içinde bulunup bulunmadığıdır. Bozuk SAHIP.toml tanı `sorunlar` listesine yazılır; tanı veri alanlarına hiç yazmaz.
