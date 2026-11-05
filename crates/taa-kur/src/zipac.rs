// Sahip: Yusuf. yazan: [Claude#taa-paket-T2 2026-11-05]
//! Zip paketi güvenli açma (T2-TASARIM PR-A2). Yol kuralları klasör paketiyle aynıdır (`paket::yol_dogrula`).
//! Sınırlar: girdi sayısı <= 10.000, bildirilen toplam açılmış boyut <= 2 GiB. Zip bombası koruması: her girdinin
//! bildirilen boyutu önce toplanır, sınır aşılırsa HİÇBİR ŞEY açılmaz; açarken gerçek bayt sayısı da sınırlanır.
//! Sembolik bağ girdileri, yinelenen adlar (harfe duyarsız) ve kökte `paket.json` olmayan zipler reddedilir.

use crate::paket::{
    dogrula_alanlar, icerik_hesapla, yol_dogrula, Paket, MAKS_DOSYA, MAKS_PAKET_JSON,
    MAKS_TOPLAM_BAYT,
};
use crate::KurHata;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::io::{BufReader, Read, Write};
use std::path::Path;

fn zip_hatasi(e: zip::result::ZipError) -> KurHata {
    KurHata::Dogrulama(format!(
        "Zip paketi okunamadı: {e}. Öneri: dosyanın bozulmadığını denetleyin, paketi yeniden üretin."
    ))
}

fn sinir_hatasi(ne: &str) -> KurHata {
    KurHata::Dogrulama(format!(
        "Zip paketi sınırı aşıyor ({ne}). Öneri: paketi küçültün ya da yeniden üretin; hiçbir şey açılmadı."
    ))
}

type Arsiv = zip::ZipArchive<BufReader<fs::File>>;

fn arsiv_ac(zip_yolu: &Path) -> Result<Arsiv, KurHata> {
    let dosya = fs::File::open(zip_yolu).map_err(|e| {
        KurHata::Yok(format!(
            "Zip dosyası açılamadı ({}): {e}. Öneri: yolu denetleyin.",
            zip_yolu.display()
        ))
    })?;
    zip::ZipArchive::new(BufReader::new(dosya)).map_err(zip_hatasi)
}

/// Birinci geçiş: hiçbir şey yazmadan tüm girdileri denetler (yol, yinelenen ad, sembolik bağ, kökte paket.json,
/// sınırlar) ve bildirilen boyutları döner.
fn tara(arsiv: &mut Arsiv, maks_dosya: usize, maks_bayt: u64) -> Result<Vec<u64>, KurHata> {
    let n = arsiv.len();
    if n > maks_dosya {
        return Err(sinir_hatasi(&format!("girdi sayısı {n} > {maks_dosya}")));
    }

    // Birinci geçiş: hiçbir şey yazmadan tüm girdileri denetle ve bildirilen boyutları topla.
    let mut boyutlar = Vec::with_capacity(n);
    let mut gorulen = BTreeSet::new();
    let mut toplam = 0u64;
    let mut paket_json_var = false;
    for i in 0..n {
        let g = arsiv.by_index_raw(i).map_err(zip_hatasi)?;
        let ad = g.name().map_err(zip_hatasi)?.to_string();
        let dizin = g.is_dir();
        let temiz = if dizin {
            ad.strip_suffix('/').unwrap_or(&ad)
        } else {
            ad.as_str()
        };
        yol_dogrula(temiz).map_err(|m| {
            KurHata::Dogrulama(format!(
                "Zip girdisi geçersiz: {m}. Öneri: paketi yeniden üretin."
            ))
        })?;
        if !gorulen.insert(temiz.to_lowercase()) {
            return Err(KurHata::Dogrulama(format!(
                "Zip içinde yinelenen girdi: {temiz}. Öneri: paketi yeniden üretin."
            )));
        }
        if let Some(kip) = g.unix_mode() {
            if kip & 0o170000 == 0o120000 {
                return Err(KurHata::Dogrulama(format!(
                    "Zip içinde sembolik bağ var: {temiz}. Öneri: bağları gerçek dosyaya çevirip yeniden üretin."
                )));
            }
        }
        if !dizin && temiz == "paket.json" {
            paket_json_var = true;
        }
        let boyut = if dizin { 0 } else { g.size() };
        toplam = toplam.saturating_add(boyut);
        if toplam > maks_bayt {
            return Err(sinir_hatasi(&format!(
                "açılmış toplam boyut {maks_bayt} bayt üstünde"
            )));
        }
        boyutlar.push(boyut);
    }
    if !paket_json_var {
        return Err(KurHata::Dogrulama(
            "Zip kökünde paket.json yok. Öneri: paket.json zipin kökünde olmalı (alt klasörde değil)."
                .into(),
        ));
    }

    Ok(boyutlar)
}

/// Girdiyi en çok `bildirilen` bayt okuyup SHA256'sını döner; fazlası zip bombası şüphesidir.
fn girdi_ozeti(g: &mut impl Read, temiz: &str, bildirilen: u64) -> Result<String, KurHata> {
    let mut okuyan = g.take(bildirilen.saturating_add(1));
    let mut h = Sha256::new();
    let mut tampon = vec![0u8; 1 << 16];
    let mut bu = 0u64;
    loop {
        let k = okuyan.read(&mut tampon).map_err(|e| {
            KurHata::Dogrulama(format!(
                "Zip girdisi okunamadı ({temiz}): {e}. Öneri: paketi yeniden üretin."
            ))
        })?;
        if k == 0 {
            break;
        }
        bu += k as u64;
        if bu > bildirilen {
            return Err(sinir_hatasi(&format!(
                "{temiz} bildirilen boyuttan büyük çıktı (zip bombası şüphesi)"
            )));
        }
        h.update(&tampon[..k]);
    }
    if bu != bildirilen {
        return Err(KurHata::Dogrulama(format!(
            "Zip girdisi bildirilen boyutla uyuşmuyor ({temiz}). Öneri: paketi yeniden üretin."
        )));
    }
    Ok(format!("{:x}", h.finalize()))
}

/// Zip paketini AÇMADAN doğrular: girdi kuralları ve sınırlar (`tara`), kökteki `paket.json` alanları, `icerik_sha256`
/// ve listedeki her dosyanın SHA256'sı akıştan hesaplanır. Diske hiçbir şey yazılmaz. `(paket, paket.json baytları)` döner.
pub fn zip_denetle(zip_yolu: &Path) -> Result<(Paket, Vec<u8>), KurHata> {
    let mut arsiv = arsiv_ac(zip_yolu)?;
    tara(&mut arsiv, MAKS_DOSYA, MAKS_TOPLAM_BAYT)?;
    let hata = |m: String| KurHata::Dogrulama(format!("{m}. Öneri: paketi yeniden üretin."));
    let bayt = {
        let mut g = arsiv.by_name("paket.json").map_err(zip_hatasi)?;
        if g.size() > MAKS_PAKET_JSON {
            return Err(hata("paket.json çok büyük".into()));
        }
        let mut b = Vec::with_capacity(g.size() as usize);
        g.read_to_end(&mut b)
            .map_err(|e| hata(format!("paket.json okunamadı: {e}")))?;
        b
    };
    let paket: Paket = serde_json::from_slice(&bayt).map_err(|e| {
        KurHata::Dogrulama(format!(
            "paket.json bozuk: {e}. Öneri: tools/paket_uret.py ile yeniden üretin."
        ))
    })?;
    dogrula_alanlar(&paket).map_err(hata)?;
    if icerik_hesapla(&paket.dosyalar) != paket.icerik_sha256 {
        return Err(hata("icerik_sha256 dosya listesiyle uyuşmuyor".into()));
    }
    let mut toplam = 0u64;
    for d in &paket.dosyalar {
        let mut g = arsiv
            .by_name(&d.yol)
            .map_err(|_| hata(format!("{} paket.json'da listeli ama zipte yok", d.yol)))?;
        if g.is_dir() {
            return Err(hata(format!("{} normal dosya değil", d.yol)));
        }
        let bildirilen = g.size();
        toplam = toplam.saturating_add(bildirilen);
        if toplam > MAKS_TOPLAM_BAYT {
            return Err(hata("toplam boyut sınırı (2 GiB) aşıldı".into()));
        }
        if girdi_ozeti(&mut g, &d.yol, bildirilen)? != d.sha256 {
            return Err(hata(format!("{} için sha256 uyuşmuyor", d.yol)));
        }
    }
    Ok((paket, bayt))
}

/// Zipi `hedef` klasörüne açar (`hedef` yoksa oluşturulur, boş olmalıdır). Hata olursa kısmen açılmış içerik
/// yerinde kalır; temizlik (eski/ altına taşıma) çağıranındır.
pub fn zip_ac(zip_yolu: &Path, hedef: &Path) -> Result<(), KurHata> {
    zip_ac_sinirli(zip_yolu, hedef, MAKS_DOSYA, MAKS_TOPLAM_BAYT)
}

/// `zip_ac` ile aynı; sınırlar verilir (testler küçük sınırla çalışır).
pub fn zip_ac_sinirli(
    zip_yolu: &Path,
    hedef: &Path,
    maks_dosya: usize,
    maks_bayt: u64,
) -> Result<(), KurHata> {
    let mut arsiv = arsiv_ac(zip_yolu)?;
    let boyutlar = tara(&mut arsiv, maks_dosya, maks_bayt)?;

    // İkinci geçiş: aç; gerçek bayt sayısı bildirilenle eşit olmalı, toplam sınırı aşamaz.
    fs::create_dir_all(hedef)?;
    let mut yazilan_toplam = 0u64;
    for (i, bildirilen) in boyutlar.iter().enumerate() {
        let mut g = arsiv.by_index(i).map_err(zip_hatasi)?;
        let ad = g.name().map_err(zip_hatasi)?.to_string();
        let temiz = ad.strip_suffix('/').unwrap_or(&ad);
        let mut yol = hedef.to_path_buf();
        yol.extend(temiz.split('/'));
        if g.is_dir() {
            fs::create_dir_all(&yol)?;
            continue;
        }
        if let Some(ust) = yol.parent() {
            fs::create_dir_all(ust)?;
        }
        let mut cikti = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&yol)?;
        let kalan = maks_bayt - yazilan_toplam;
        let ust_sinir = (*bildirilen).min(kalan).saturating_add(1);
        let mut okuyan = (&mut g).take(ust_sinir);
        let mut tampon = vec![0u8; 1 << 16];
        let mut bu = 0u64;
        loop {
            let k = okuyan.read(&mut tampon).map_err(|e| {
                KurHata::Dogrulama(format!(
                    "Zip girdisi okunamadı ({temiz}): {e}. Öneri: paketi yeniden üretin."
                ))
            })?;
            if k == 0 {
                break;
            }
            bu += k as u64;
            if bu > *bildirilen || yazilan_toplam + bu > maks_bayt {
                return Err(sinir_hatasi(&format!(
                    "{temiz} bildirilen boyuttan büyük çıktı (zip bombası şüphesi)"
                )));
            }
            cikti.write_all(&tampon[..k])?;
        }
        if bu != *bildirilen {
            return Err(KurHata::Dogrulama(format!(
                "Zip girdisi bildirilen boyutla uyuşmuyor ({temiz}). Öneri: paketi yeniden üretin."
            )));
        }
        yazilan_toplam += bu;
    }
    Ok(())
}
