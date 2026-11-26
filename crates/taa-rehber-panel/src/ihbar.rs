//! İhbar Hattı sayfasının sunucu tarafı. Kayıt `taa-ihbar` ile yapılır (varsayılan anonim, yalnız-ekleme defter, hız sınırı).
//! AĞ YOK: `Gonderici` bağlı değildir; iletim durumu "henüz bağlı değil" diye dürüstçe bildirilir, ihbar kaybolmaz.
//! DÜRÜST SINIR: tam anonimlik garanti edilmez; ihbar metninin içeriği kişiyi ele verebilir.
//! Sahip: Yusuf. yazan: [Claude#rehber-panel 2026-11-26]

use crate::durum::{Durum, Ortam};
use crate::{zaman, Yanit};
use serde_json::{json, Value};
use taa_ihbar::{
    takip_no_uret, Defter, Girdi, Gonderici, Hat, Hata, Ihbar, Kategori, Teslim, METIN_EN_COK,
};

/// İhbarı bu bilgisayardaki hız sınırı için tek kaynak sayar (kaynak hiçbir yere yazılmaz).
const KAYNAK: &str = "yerel-panel";

pub const BAGLI_DEGIL: &str = "gönderim henüz bağlı değil";

/// Gerçek Telegram/e-posta göndericisi bu sürümde bağlı değildir: her gönderim "bağlı değil" hatası döner.
pub struct BagliDegil;

impl Gonderici for BagliDegil {
    fn telegram_gonder(&self, _: &str, _: &str) -> Result<(), String> {
        Err(BAGLI_DEGIL.into())
    }
    fn eposta_gonder(&self, _: &str, _: &str, _: &str) -> Result<(), String> {
        Err(BAGLI_DEGIL.into())
    }
}

pub fn kategoriler() -> Value {
    let k = [
        Kategori::Hirsizlik,
        Kategori::Usulsuzluk,
        Kategori::Taciz,
        Kategori::Guvenlik,
        Kategori::Diger,
    ];
    let kod = |k: Kategori| match k {
        Kategori::Hirsizlik => "hirsizlik",
        Kategori::Usulsuzluk => "usulsuzluk",
        Kategori::Taciz => "taciz",
        Kategori::Guvenlik => "guvenlik",
        Kategori::Diger => "diger",
    };
    json!({
        "kategoriler": k.iter().map(|x| json!({"kod": kod(*x), "etiket": x.etiket()})).collect::<Vec<_>>(),
        "metin_en_cok": METIN_EN_COK,
        "iletim_bagli": false,
    })
}

fn teslim_kodu(t: &Teslim) -> &'static str {
    match t {
        Teslim::Gitti => "gitti",
        Teslim::Kapali => "kapali",
        Teslim::Hata(_) => "hata",
    }
}

fn hata_yaniti(h: Hata) -> Yanit {
    match h {
        Hata::Kapali => (
            503,
            json!({"hata": "İhbar hattı bu bilgisayarda kapalı.", "kod": "kapali"}),
        ),
        Hata::HizSiniri { bekle_sn, genel } => (
            429,
            json!({"hata": h_metni(bekle_sn), "kod": "hiz", "bekle_sn": bekle_sn, "genel": genel}),
        ),
        Hata::Gecersiz(m) => (400, json!({"hata": m, "kod": "gecersiz"})),
        // Yol ya da sistem ayrıntısı ekrana sızmasın.
        Hata::Depo(_) => (
            500,
            json!({"hata": "İhbar kaydedilemedi. Öneri: veri klasörü yazılabilir mi bakın.", "kod": "depo"}),
        ),
    }
}

fn h_metni(bekle: u64) -> String {
    format!("Çok sık ihbar gönderildi; {bekle} sn sonra yeniden deneyin.")
}

/// `POST /api/ihbar {kategori, metin, anonim?, iletisim?}`. `anonim` varsayılan `true`; anonimken iletişim yok sayılır.
pub fn gonder(d: &mut Durum, o: &dyn Ortam, govde: &Value) -> Yanit {
    let kategori = match govde
        .get("kategori")
        .and_then(Value::as_str)
        .and_then(Kategori::koddan)
    {
        Some(k) => k,
        None => {
            return (
                400,
                json!({"hata": "Kategori seçin.", "kod": "gecersiz", "alan": "kategori"}),
            )
        }
    };
    let Some(metin) = govde.get("metin").and_then(Value::as_str) else {
        return (
            400,
            json!({"hata": "Açıklama yazın.", "kod": "gecersiz", "alan": "metin"}),
        );
    };
    let anonim = govde.get("anonim").and_then(Value::as_bool).unwrap_or(true);
    let iletisim = if anonim {
        None
    } else {
        govde
            .get("iletisim")
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    let girdi = Girdi {
        kategori,
        metin: metin.to_string(),
        iletisim,
    };
    let mut tohum = [0u8; 16];
    if o.rastgele(&mut tohum).is_err() {
        return (
            500,
            json!({"hata": "Rastgele sayı alınamadı.", "kod": "depo"}),
        );
    }
    let simdi = o.simdi();
    let tarih = zaman::tarih(simdi);
    let defter = Defter {
        yol: &d.ihbar.defter_yolu,
    };
    let (takip_no, telegram, eposta) = match &d.ihbar.ayar {
        Some(ayar) => {
            let g = BagliDegil;
            let hat = Hat {
                defter: &defter,
                ayar,
                gonderici: &g,
            };
            match hat.al(&mut d.ihbar.hiz, KAYNAK, girdi, &tarih, simdi, &tohum) {
                Ok(m) => (m.takip_no, m.telegram, m.eposta),
                Err(h) => return hata_yaniti(h),
            }
        }
        // Ayar yok: iletim kanalı tanımlı değil; ihbar yine de deftere yazılır (kayıp olmaz).
        None => match ayarsiz_kaydet(&defter, &mut d.ihbar.hiz, girdi, &tarih, simdi, &tohum) {
            Ok(n) => (n, Teslim::Kapali, Teslim::Kapali),
            Err(h) => return hata_yaniti(h),
        },
    };
    let iletildi = telegram == Teslim::Gitti || eposta == Teslim::Gitti;
    let mesaj = if iletildi {
        "İhbarınız alındı ve iletildi.".to_string()
    } else if d.ihbar.ayar.is_some() {
        format!("İhbarınız kaydedildi; {BAGLI_DEGIL}: bildirim yetkiliye henüz ulaşmadı, kayıt bu bilgisayarda saklanıyor.")
    } else {
        "İhbarınız kaydedildi; iletim kanalı ayarlanmamış (ihbar.json yok) ve gönderim henüz bağlı değil: bildirim yetkiliye henüz ulaşmadı, kayıt bu bilgisayarda saklanıyor.".to_string()
    };
    (
        201,
        json!({
            "alindi": true,
            "takip_no": takip_no,
            "anonim": anonim,
            "iletildi": iletildi,
            "telegram": teslim_kodu(&telegram),
            "eposta": teslim_kodu(&eposta),
            "mesaj": mesaj,
        }),
    )
}

fn ayarsiz_kaydet(
    defter: &Defter,
    hiz: &mut taa_ihbar::HizSiniri,
    girdi: Girdi,
    tarih: &str,
    simdi: u64,
    tohum: &[u8],
) -> Result<String, Hata> {
    hiz.dene(KAYNAK, simdi).map_err(|b| Hata::HizSiniri {
        bekle_sn: b.saniye,
        genel: b.genel,
    })?;
    for deneme in 0..16u64 {
        let no = takip_no_uret(tohum, tarih, deneme).map_err(Hata::Gecersiz)?;
        let ihbar = Ihbar::olustur(girdi.clone(), tarih, no.clone()).map_err(Hata::Gecersiz)?;
        match defter.ekle(&ihbar) {
            Ok(()) => {
                defter
                    .teslimat_yaz(&no, tarih, Teslim::Kapali, Teslim::Kapali)
                    .map_err(Hata::Depo)?;
                return Ok(no);
            }
            Err(e) if e.contains("takip numarası defterde var") => continue,
            Err(e) => return Err(Hata::Depo(e)),
        }
    }
    Err(Hata::Depo("takip numarası üretilemedi".into()))
}
