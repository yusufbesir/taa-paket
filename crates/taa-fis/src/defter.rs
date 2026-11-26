//! Fiş defteri (yalnız eklemeli JSONL). lib.rs'den ayrıldı; davranış aynı.

use super::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize)]
#[serde(tag = "tur", content = "v", rename_all = "lowercase")]
#[allow(clippy::large_enum_variant)]
pub(crate) enum Kayit {
    Fis(Fis),
    Durum {
        no: String,
        durum: Durum,
        tarih: String,
    },
    Teslim {
        no: String,
        degisiklik: TeslimDegisti,
        saat: String,
    },
    Raf {
        no: String,
        raf: String,
        kim: String,
        tarih: String,
    },
    Imza {
        no: String,
        imza: Imza,
    },
    /// Sonradan kalem/işlem ekleme (ör. ütü istendi).
    KalemEkle {
        no: String,
        kalem: Kalem,
        kim: String,
        tarih: String,
    },
    /// Kalem çıkarma (sıra 0'dan); neden yazılır.
    KalemCikar {
        no: String,
        sira: usize,
        neden: String,
        kim: String,
        tarih: String,
    },
    /// Teslim şekli: kargo ↔ mağazadan teslim, ya da kargo bilgisi düzeltmesi.
    TeslimSekli {
        no: String,
        kargo: bool,
        #[serde(default)]
        kargo_ad: String,
        #[serde(default)]
        kargo_adres: String,
        #[serde(default)]
        kargo_telefon: String,
        #[serde(default)]
        kargo_notu: String,
        #[serde(default)]
        neden: String,
        kim: String,
        tarih: String,
    },
    /// İki fişi birlikte teslim için bağlar (gruplar birleşir).
    Bag {
        no: String,
        diger: String,
        kim: String,
        tarih: String,
    },
    /// Fişi bağlı olduğu gruptan çıkarır.
    BagCoz {
        no: String,
        kim: String,
        tarih: String,
    },
    /// Zincir başlangıcı (TT1): bu satırdan sonraki satırlar zincire girer. Eski satırlar yeniden yazılmaz.
    ZincirBasi {
        tarih: String,
    },
}

/// Defterin zincir ucu: kaç satır (başlangıç dahil) ve o satıra kadarki özet (sha-256, onaltılık).
/// Zincir kısmi değişikliği yakalar; diske erişen birinin dosyanın tamamını yeniden yazmasını tek başına
/// önlemez, bu yüzden uç eşitlemede (TT3) merkeze gönderilir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZincirUcu {
    pub sayi: usize,
    pub ozet: String,
}

fn ozet_ileri(onceki: &str, satir: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(onceki.as_bytes());
    h.update(b"\n");
    h.update(satir.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Düzeltme geçmişinde ve fişte bir kalemin kısa yazımı: "2 x Pantolon (Siyah) - Paça kısalt".
pub fn kalem_ozeti(k: &Kalem) -> String {
    let mut s = String::new();
    if let Some(a) = k.adet.filter(|a| *a > 1) {
        s.push_str(&format!("{a} x "));
    }
    s.push_str(k.urun.trim());
    if let Some(r) = k.renk.as_deref().and_then(renk_adi) {
        s.push_str(&format!(" ({r})"));
    }
    s.push_str(&format!(" - {}", k.islem.trim()));
    s
}

pub struct Defter<'a> {
    pub yol: &'a Path,
}

pub(crate) fn kapali() -> Result<(), String> {
    if taa_ayar::ozellik_acik(OZELLIK, true) {
        Ok(())
    } else {
        Err(taa_ayar::kapali_mesaji(OZELLIK))
    }
}

impl Defter<'_> {
    fn kayitlar(&self) -> Result<Vec<Kayit>, String> {
        taa_dosya::satirlari_oku(self.yol)
            .map_err(|e| e.to_string())?
            .iter()
            .map(|l| serde_json::from_str(l).map_err(|e| format!("defter satırı bozuk: {e}")))
            .collect()
    }

    /// Fişlerin güncel durumları (defter sırasıyla).
    pub fn fisler(&self) -> Result<Vec<Fis>, String> {
        let mut f: Vec<Fis> = Vec::new();
        for k in self.kayitlar()? {
            match k {
                Kayit::Fis(x) => f.push(x),
                Kayit::Durum { no, durum, .. } => {
                    if let Some(x) = f.iter_mut().find(|x| x.no == no) {
                        x.durum = durum;
                    }
                }
                Kayit::Teslim {
                    no,
                    degisiklik,
                    saat,
                } => {
                    if let Some(x) = f.iter_mut().find(|x| x.no == no) {
                        x.teslim = degisiklik.yeni.clone();
                        x.teslim_saati = saat;
                        x.teslim_degisiklikleri.push(degisiklik);
                    }
                }
                Kayit::Raf { no, raf, .. } => {
                    if let Some(x) = f.iter_mut().find(|x| x.no == no) {
                        x.raf = raf;
                    }
                }
                Kayit::Imza { no, imza } => {
                    if let Some(x) = f.iter_mut().find(|x| x.no == no) {
                        x.imzalar.push(imza);
                    }
                }
                Kayit::KalemEkle {
                    no,
                    kalem,
                    kim,
                    tarih,
                } => {
                    if let Some(x) = f.iter_mut().find(|x| x.no == no) {
                        x.duzeltmeler.push(Duzeltme {
                            ne: "kalem-ekle".into(),
                            ayrinti: kalem_ozeti(&kalem),
                            kim,
                            tarih,
                        });
                        x.kalemler.push(kalem);
                    }
                }
                Kayit::KalemCikar {
                    no,
                    sira,
                    neden,
                    kim,
                    tarih,
                } => {
                    if let Some(x) = f.iter_mut().find(|x| x.no == no) {
                        if sira < x.kalemler.len() {
                            let k = x.kalemler.remove(sira);
                            let mut ayrinti = kalem_ozeti(&k);
                            if !neden.is_empty() {
                                ayrinti.push_str(&format!(" (neden: {neden})"));
                            }
                            x.duzeltmeler.push(Duzeltme {
                                ne: "kalem-cikar".into(),
                                ayrinti,
                                kim,
                                tarih,
                            });
                        }
                    }
                }
                Kayit::TeslimSekli {
                    no,
                    kargo,
                    kargo_ad,
                    kargo_adres,
                    kargo_telefon,
                    kargo_notu,
                    neden,
                    kim,
                    tarih,
                } => {
                    if let Some(x) = f.iter_mut().find(|x| x.no == no) {
                        let mut ayrinti = match (x.kargo, kargo) {
                            (false, true) => "mağazadan teslim → kargo".to_string(),
                            (true, false) => "kargo iptal → mağazadan teslim".to_string(),
                            _ if kargo => "kargo bilgisi düzeltildi".to_string(),
                            _ => "mağazadan teslim".to_string(),
                        };
                        if !neden.is_empty() {
                            ayrinti.push_str(&format!(" (neden: {neden})"));
                        }
                        x.kargo = kargo;
                        x.kargo_ad = kargo_ad;
                        x.kargo_adres = kargo_adres;
                        x.kargo_telefon = kargo_telefon;
                        x.kargo_notu = kargo_notu;
                        x.duzeltmeler.push(Duzeltme {
                            ne: "teslim-sekli".into(),
                            ayrinti,
                            kim,
                            tarih,
                        });
                    }
                }
                Kayit::Bag {
                    no,
                    diger,
                    kim,
                    tarih,
                } => {
                    let grup = |f: &Vec<Fis>, n: &str| -> Vec<String> {
                        let mut g: Vec<String> = f
                            .iter()
                            .find(|x| x.no == n)
                            .map(|x| x.bagli.clone())
                            .unwrap_or_default();
                        g.push(n.to_string());
                        g
                    };
                    let mut hepsi = grup(&f, &no);
                    for n in grup(&f, &diger) {
                        if !hepsi.contains(&n) {
                            hepsi.push(n);
                        }
                    }
                    for x in f.iter_mut().filter(|x| hepsi.contains(&x.no)) {
                        x.bagli = hepsi.iter().filter(|n| **n != x.no).cloned().collect();
                        let karsi = if x.no == no { &diger } else { &no };
                        x.duzeltmeler.push(Duzeltme {
                            ne: "bagla".into(),
                            ayrinti: format!("{karsi} ile birlikte teslim"),
                            kim: kim.clone(),
                            tarih: tarih.clone(),
                        });
                    }
                }
                Kayit::BagCoz { no, kim, tarih } => {
                    let eski = f
                        .iter()
                        .find(|x| x.no == no)
                        .map(|x| x.bagli.clone())
                        .unwrap_or_default();
                    for x in f.iter_mut() {
                        if x.no == no {
                            x.bagli.clear();
                            x.duzeltmeler.push(Duzeltme {
                                ne: "bag-coz".into(),
                                ayrinti: "bağlı fişlerden ayrıldı".into(),
                                kim: kim.clone(),
                                tarih: tarih.clone(),
                            });
                        } else if eski.contains(&x.no) {
                            x.bagli.retain(|n| *n != no);
                        }
                    }
                }
                Kayit::ZincirBasi { .. } => {}
            }
        }
        Ok(f)
    }

    /// Ürünün stant/raf yerini yazar (QR okutulunca). Eski yer silinmez, defterde kalır.
    pub fn raf_ata(&self, no: &str, raf: &str, kim: &str, tarih: &str) -> Result<(), String> {
        kapali()?;
        if !taa_ayar::ozellik_acik("fis-raf", true) {
            return Err(taa_ayar::kapali_mesaji("fis-raf"));
        }
        let raf = raf.trim().to_uppercase();
        if raf.is_empty()
            || raf.chars().count() > 20
            || !raf
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == ' ')
        {
            return Err("stant/raf kodu 1-20 harf, rakam ya da tire olmalı (örn. A-12)".into());
        }
        let f = self.bul(no)?.ok_or("fiş yok")?;
        if matches!(f.durum, Durum::Teslim | Durum::Iptal) {
            return Err("teslim edilmiş ya da iptal fişe raf yazılamaz".into());
        }
        let s = serde_json::to_string(&Kayit::Raf {
            no: no.into(),
            raf,
            kim: kim.trim().into(),
            tarih: tarih.into(),
        })
        .map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &s).map_err(|e| e.to_string())
    }

    /// Telefondaki kullanıcı adına imza atar (ör. usta işi bitirince). Aynı kişi aynı rolde bir kez imzalar.
    pub fn imzala(&self, no: &str, kim: &str, rol: &str, tarih: &str) -> Result<(), String> {
        self.imzala_ayrintili(no, kim, rol, tarih, "", "")
    }

    /// `imzala` + imza saati ve cihaz kimliği (TT1: olay kimliği). Aynı kural geçerli.
    pub fn imzala_ayrintili(
        &self,
        no: &str,
        kim: &str,
        rol: &str,
        tarih: &str,
        saat: &str,
        cihaz: &str,
    ) -> Result<(), String> {
        self.imzala_kisili(no, kim, rol, tarih, saat, cihaz, "dogrulanmis", "")
    }

    /// Kimlik düzeyi ve okutan kişi kimliğiyle imza. "beyan" düzeyinde kişi kimliği zorunlu;
    /// mağaza telefonu (rol teslim-eden/teslim-alan dışı) usta imzasını yalnız "dogrulanmis" atabilir.
    #[allow(clippy::too_many_arguments)]
    pub fn imzala_kisili(
        &self,
        no: &str,
        kim: &str,
        rol: &str,
        tarih: &str,
        saat: &str,
        cihaz: &str,
        kimlik_duzeyi: &str,
        okutan_kisi_id: &str,
    ) -> Result<(), String> {
        if !["dogrulanmis", "beyan"].contains(&kimlik_duzeyi) {
            return Err("kimlik düzeyi dogrulanmis ya da beyan olmalı".into());
        }
        if kimlik_duzeyi == "beyan" && okutan_kisi_id.trim().is_empty() {
            return Err("beyan imzasında okutan kişi seçilmeli".into());
        }
        if kimlik_duzeyi == "beyan" && rol.trim() == "usta" {
            return Err("usta imzası yalnız doğrulanmış kimlikle atılır".into());
        }
        kapali()?;
        if !taa_ayar::ozellik_acik("fis-imza", true) {
            return Err(taa_ayar::kapali_mesaji("fis-imza"));
        }
        let (kim, rol) = (kim.trim(), rol.trim());
        if kim.is_empty() || !["usta", "teslim-eden", "teslim-alan"].contains(&rol) {
            return Err("imza: kullanıcı ve rol (usta, teslim-eden, teslim-alan) gerekli".into());
        }
        let f = self.bul(no)?.ok_or("fiş yok")?;
        if f.durum == Durum::Iptal {
            return Err("iptal fiş imzalanamaz".into());
        }
        if f.imzalar.iter().any(|i| i.kim == kim && i.rol == rol) {
            return Err(format!("{kim} bu fişi '{rol}' olarak zaten imzaladı"));
        }
        let s = serde_json::to_string(&Kayit::Imza {
            no: no.into(),
            imza: Imza {
                kim: kim.into(),
                rol: rol.into(),
                tarih: tarih.into(),
                saat: saat.trim().into(),
                cihaz: cihaz.trim().into(),
                kimlik_duzeyi: kimlik_duzeyi.into(),
                okutan_kisi_id: okutan_kisi_id.trim().into(),
            },
        })
        .map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &s).map_err(|e| e.to_string())
    }

    /// Zinciri başlatır (bir kez). Bundan sonra eklenen her satır zincire girer; eskiler yeniden yazılmaz.
    pub fn zincir_baslat(&self, tarih: &str) -> Result<(), String> {
        kapali()?;
        if !tarih_gecerli(tarih) {
            return Err("tarih YYYY-AA-GG olmalı".into());
        }
        if self.zincir_ucu()?.is_some() {
            return Err("zincir zaten başlatılmış".into());
        }
        let s = serde_json::to_string(&Kayit::ZincirBasi {
            tarih: tarih.into(),
        })
        .map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &s).map_err(|e| e.to_string())
    }

    /// Her zincir satırından sonraki özetler (başlangıç satırı dahil); zincir yoksa boş.
    fn zincir_ozetleri(&self) -> Result<Vec<String>, String> {
        let satirlar = taa_dosya::satirlari_oku(self.yol).map_err(|e| e.to_string())?;
        let mut ozetler: Vec<String> = Vec::new();
        for l in &satirlar {
            if ozetler.is_empty() {
                if matches!(
                    serde_json::from_str::<Kayit>(l),
                    Ok(Kayit::ZincirBasi { .. })
                ) {
                    ozetler.push(ozet_ileri("TAA-ZINCIR-1", l));
                }
            } else {
                let son = ozetler[ozetler.len() - 1].clone();
                ozetler.push(ozet_ileri(&son, l));
            }
        }
        Ok(ozetler)
    }

    /// Güncel zincir ucu; zincir başlamadıysa `None`.
    pub fn zincir_ucu(&self) -> Result<Option<ZincirUcu>, String> {
        let o = self.zincir_ozetleri()?;
        Ok(o.last().map(|ozet| ZincirUcu {
            sayi: o.len(),
            ozet: ozet.clone(),
        }))
    }

    /// Merkeze daha önce gönderilmiş uç hâlâ defterin içinde mi? Zincirin o noktaya kadarki satırları
    /// değişmediyse `true`; satır değişmiş, silinmiş ya da defter kısalmışsa `false`.
    pub fn zincir_dogrula(&self, bilinen: &ZincirUcu) -> Result<bool, String> {
        let o = self.zincir_ozetleri()?;
        Ok(bilinen.sayi >= 1 && o.get(bilinen.sayi - 1) == Some(&bilinen.ozet))
    }

    pub fn bul(&self, no: &str) -> Result<Option<Fis>, String> {
        Ok(self.fisler()?.into_iter().find(|f| f.no == no))
    }

    pub fn ekle(&self, g: Girdi) -> Result<Fis, String> {
        self.ekle_uyarili(g).map(|(f, _)| f)
    }

    /// Fişi ekler; kalem uyarılarını da döndürür. Kurallar `terzi-kurallari.json` (defterin klasöründe) ya da gömülü katalogdur.
    pub fn ekle_uyarili(&self, g: Girdi) -> Result<(Fis, Vec<String>), String> {
        kapali()?;
        let katalog = taa_terzi::Katalog::dizinden(self.yol.parent().unwrap_or(Path::new(".")))?;
        let (telefon, uyarilar) = dogrula(
            &g,
            &katalog,
            taa_ayar::ozellik_acik("serbest-islem", true),
            taa_ayar::ozellik_acik("olcu-zorunlu", false),
        )?;
        let kargo = g.kargo && taa_ayar::ozellik_acik("kargo-alani", true);
        let mut kargo_telefon = String::new();
        if kargo {
            if g.kargo_ad.split_whitespace().count() < 2 {
                return Err("kargo: alıcının ad ve soyadı yazılmalı".into());
            }
            if g.kargo_adres.trim().chars().count() < 15 {
                return Err("kargo: açık adres (mahalle, sokak, no, ilçe/il) yazılmalı".into());
            }
            kargo_telefon = telefon_duzelt(&g.kargo_telefon)
                .map_err(|_| "kargo: alıcı telefon numarası geçerli değil".to_string())?;
        }
        let gun = g.tarih.replace('-', "");
        let sira = self.fisler()?.iter().filter(|f| f.tarih == g.tarih).count() + 1;
        let fis = Fis {
            no: format!("F-{gun}-{sira:03}"),
            tarih: g.tarih,
            musteri: g.musteri.trim().into(),
            telefon,
            kalemler: g.kalemler,
            olcu_notu: g.olcu_notu.trim().into(),
            prova: g.prova,
            teslim: g.teslim,
            teslim_saati: g.teslim_saati,
            ucret_kurus: g.ucret_kurus,
            on_odeme_kurus: g.on_odeme_kurus,
            usta: g.usta.trim().into(),
            satis_danismani: g.satis_danismani.trim().into(),
            olcu_alan: g.olcu_alan.trim().into(),
            satis_danismani_id: g.satis_danismani_id.trim().into(),
            olcu_alan_id: g.olcu_alan_id.trim().into(),
            duzenleyen_id: g.duzenleyen_id.trim().into(),
            kargo,
            kargo_notu: g.kargo_notu.trim().into(),
            kargo_ad: if kargo {
                g.kargo_ad.trim().into()
            } else {
                String::new()
            },
            kargo_adres: if kargo {
                g.kargo_adres.trim().into()
            } else {
                String::new()
            },
            kargo_telefon,
            bildirim_izni: g.bildirim_izni,
            teslim_degisiklikleri: Vec::new(),
            durum: Durum::Bekliyor,
            raf: String::new(),
            imzalar: Vec::new(),
            sube: g.sube.trim().into(),
            duzeltmeler: Vec::new(),
            bagli: Vec::new(),
        };
        let s = serde_json::to_string(&Kayit::Fis(fis.clone())).map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &s).map_err(|e| e.to_string())?;
        Ok((fis, uyarilar))
    }

    /// Teslim tarihini/saatini değiştirir; eski değer silinmez, neden zorunludur.
    pub fn teslim_degistir(
        &self,
        no: &str,
        yeni: &str,
        yeni_saat: &str,
        neden: &str,
        tarih: &str,
    ) -> Result<(), String> {
        kapali()?;
        let f = self.bul(no)?.ok_or_else(|| format!("fiş yok: {no}"))?;
        if matches!(f.durum, Durum::Teslim | Durum::Iptal) {
            return Err("teslim edilmiş ya da iptal fişin teslim tarihi değiştirilemez".into());
        }
        if !tarih_gecerli(yeni) || !tarih_gecerli(tarih) || !saat_gecerli(yeni_saat) {
            return Err("tarih YYYY-AA-GG, saat SS:DD olmalı".into());
        }
        if yeni < f.tarih.as_str() {
            return Err("yeni teslim tarihi fiş tarihinden önce olamaz".into());
        }
        if neden.trim().is_empty() {
            return Err("değişiklik nedeni yazılmalı".into());
        }
        let s = serde_json::to_string(&Kayit::Teslim {
            no: no.into(),
            degisiklik: TeslimDegisti {
                eski: f.teslim,
                yeni: yeni.into(),
                neden: neden.trim().into(),
                tarih: tarih.into(),
            },
            saat: yeni_saat.into(),
        })
        .map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &s).map_err(|e| e.to_string())
    }

    /// Durumu ilerletir: Bekliyor -> (Terzihanede) -> Hazir -> Teslim; Iptal Teslim'den önce. Geri dönüş yok.
    pub fn durum_ata(&self, no: &str, yeni: Durum, tarih: &str) -> Result<(), String> {
        kapali()?;
        let f = self.bul(no)?.ok_or_else(|| format!("fiş yok: {no}"))?;
        let izin = matches!(
            (f.durum, yeni),
            (Durum::Bekliyor, Durum::Terzihanede | Durum::Hazir)
                | (Durum::Terzihanede, Durum::Hazir)
                | (Durum::Hazir, Durum::Teslim)
                | (
                    Durum::Bekliyor | Durum::Terzihanede | Durum::Hazir,
                    Durum::Iptal
                )
        );
        if !izin {
            return Err(format!(
                "{:?} durumundan {:?} durumuna geçilemez",
                f.durum, yeni
            ));
        }
        if !tarih_gecerli(tarih) {
            return Err("tarih YYYY-AA-GG olmalı".into());
        }
        let s = serde_json::to_string(&Kayit::Durum {
            no: no.into(),
            durum: yeni,
            tarih: tarih.into(),
        })
        .map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &s).map_err(|e| e.to_string())
    }

    fn duzeltilebilir(&self, no: &str) -> Result<Fis, String> {
        kapali()?;
        let f = self.bul(no)?.ok_or_else(|| format!("fiş yok: {no}"))?;
        if matches!(f.durum, Durum::Teslim | Durum::Iptal) {
            return Err("teslim edilmiş ya da iptal fiş düzeltilemez".into());
        }
        Ok(f)
    }

    fn yaz(&self, k: &Kayit) -> Result<(), String> {
        let s = serde_json::to_string(k).map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &s).map_err(|e| e.to_string())
    }

    /// Fişe sonradan kalem ekler (ör. ütü istendi). Fiş yazılırkenki kalem kuralı geçerli; uyarıları döndürür.
    pub fn kalem_ekle(
        &self,
        no: &str,
        kalem: Kalem,
        kim: &str,
        tarih: &str,
    ) -> Result<Vec<String>, String> {
        self.duzeltilebilir(no)?;
        let katalog = taa_terzi::Katalog::dizinden(self.yol.parent().unwrap_or(Path::new(".")))?;
        let uyarilar = kalem_denetle(
            &kalem,
            &katalog,
            taa_ayar::ozellik_acik("serbest-islem", true),
            taa_ayar::ozellik_acik("olcu-zorunlu", false),
        )?;
        if !tarih_gecerli(tarih) {
            return Err("tarih YYYY-AA-GG olmalı".into());
        }
        self.yaz(&Kayit::KalemEkle {
            no: no.into(),
            kalem,
            kim: kim.trim().into(),
            tarih: tarih.into(),
        })?;
        Ok(uyarilar)
    }

    /// Kalemi fişten çıkarır (sıra 0'dan). Son kalem çıkarılamaz (fişi iptal edin).
    pub fn kalem_cikar(
        &self,
        no: &str,
        sira: usize,
        neden: &str,
        kim: &str,
        tarih: &str,
    ) -> Result<(), String> {
        let f = self.duzeltilebilir(no)?;
        if sira >= f.kalemler.len() {
            return Err("böyle bir kalem yok".into());
        }
        if f.kalemler.len() == 1 {
            return Err("fişte tek kalem var; çıkarmak yerine fişi iptal edin".into());
        }
        self.yaz(&Kayit::KalemCikar {
            no: no.into(),
            sira,
            neden: neden.trim().chars().take(200).collect(),
            kim: kim.trim().into(),
            tarih: tarih.into(),
        })
    }

    /// Teslim şeklini değiştirir: kargo ↔ mağazadan teslim, ya da kargo bilgisini düzeltir.
    /// Kargoda alıcı ad-soyad, açık adres ve geçerli telefon zorunlu (fiş yazılırkenki kural).
    #[allow(clippy::too_many_arguments)]
    pub fn teslim_sekli(
        &self,
        no: &str,
        kargo: bool,
        kargo_ad: &str,
        kargo_adres: &str,
        kargo_telefon: &str,
        kargo_notu: &str,
        neden: &str,
        kim: &str,
        tarih: &str,
    ) -> Result<(), String> {
        self.duzeltilebilir(no)?;
        let mut tel = String::new();
        if kargo {
            if kargo_ad.split_whitespace().count() < 2 {
                return Err("kargo: alıcının ad ve soyadı yazılmalı".into());
            }
            if kargo_adres.trim().chars().count() < 15 {
                return Err("kargo: açık adres (mahalle, sokak, no, ilçe/il) yazılmalı".into());
            }
            tel = telefon_duzelt(kargo_telefon)
                .map_err(|_| "kargo: alıcı telefon numarası geçerli değil".to_string())?;
        }
        let (ad, adres, notu) = if kargo {
            (
                kargo_ad.trim().into(),
                kargo_adres.trim().into(),
                kargo_notu.trim().into(),
            )
        } else {
            (String::new(), String::new(), String::new())
        };
        self.yaz(&Kayit::TeslimSekli {
            no: no.into(),
            kargo,
            kargo_ad: ad,
            kargo_adres: adres,
            kargo_telefon: tel,
            kargo_notu: notu,
            neden: neden.trim().chars().take(200).collect(),
            kim: kim.trim().into(),
            tarih: tarih.into(),
        })
    }

    /// İki fişi birlikte teslim için bağlar (ör. birlikte alışveriş yapan arkadaşlar). Teslimde bağlı fişler sorulur.
    pub fn bagla(&self, no: &str, diger: &str, kim: &str, tarih: &str) -> Result<(), String> {
        let f = self.duzeltilebilir(no)?;
        if no == diger {
            return Err("fiş kendisine bağlanamaz".into());
        }
        self.duzeltilebilir(diger)?;
        if f.bagli.iter().any(|n| n == diger) {
            return Err("bu fişler zaten bağlı".into());
        }
        self.yaz(&Kayit::Bag {
            no: no.into(),
            diger: diger.into(),
            kim: kim.trim().into(),
            tarih: tarih.into(),
        })
    }

    /// Fişi bağlı grubundan çıkarır.
    pub fn bag_coz(&self, no: &str, kim: &str, tarih: &str) -> Result<(), String> {
        let f = self.bul(no)?.ok_or_else(|| format!("fiş yok: {no}"))?;
        if f.bagli.is_empty() {
            return Err("fiş başka fişe bağlı değil".into());
        }
        self.yaz(&Kayit::BagCoz {
            no: no.into(),
            kim: kim.trim().into(),
            tarih: tarih.into(),
        })
    }

    /// Durum değişikliklerinin tarihli geçmişi: (fiş no, yeni durum, tarih), defter sırasıyla.
    /// Takip raporları (usta zamanında bitirme vb.) bunu kullanır.
    pub fn durum_gecmisi(&self) -> Result<Vec<(String, Durum, String)>, String> {
        Ok(self
            .kayitlar()?
            .into_iter()
            .filter_map(|k| match k {
                Kayit::Durum { no, durum, tarih } => Some((no, durum, tarih)),
                _ => None,
            })
            .collect())
    }

    /// Teslim tarihi geçmiş ve teslim edilmemiş fişler.
    pub fn geciken(&self, bugun: &str) -> Result<Vec<Fis>, String> {
        Ok(self
            .fisler()?
            .into_iter()
            .filter(|f| {
                matches!(f.durum, Durum::Bekliyor | Durum::Terzihanede | Durum::Hazir)
                    && f.teslim.as_str() < bugun
            })
            .collect())
    }
}
