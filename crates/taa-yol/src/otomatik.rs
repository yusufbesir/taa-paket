// Sahip: Yusuf. yazan: [Claude#dinamik-adres 2026-11-25]
//! Dinamik sunucu adresi (Yusuf 2026-11-25): sunucu dizüstü dolaşır, LAN adresi değişir. Sunucu o anki adreslerini
//! `GET /v1/yol-bilgisi` ile bildirir (`{"adresler":["https://192.168.1.250","https://desktop-mr12inl"]}`);
//! istemci bunları `oto-1`, `oto-2`… adlı yollar olarak listenin BAŞINA koyar (en hızlı yol LAN).
//!
//! Güvenlik: duyurulan adres yalnız **özel/yerel** olabilir (RFC1918, Tailscale 100.64/10, link-local, `.local`,
//! `.ts.net`, tek etiketli bilgisayar adı); genel IP/alan adı duyurusu yok sayılır. Duyurulan yol kendi CA'mızla
//! (`sunucu-ca.pem`) doğrulanır, yani yanıtı yalnız sunucu (TLS + CA) verebilir. Elle eklenen yollara (`oto-` ile
//! başlamayanlara) dokunulmaz; her güncellemede yalnız `oto-*` yollar yeniden yazılır.

use crate::{Yol, YolHata, Yollar};
use serde_json::Value;
use std::net::Ipv4Addr;
use taa_kuyruk::{GonderHata, Gonderici};

pub const BILGI_UCU: &str = "/v1/yol-bilgisi";
pub const OTO_ON_EK: &str = "oto-";
/// Bir güncellemede en çok bu kadar adres alınır.
pub const OTO_EN_COK: usize = 4;

fn ozel_ipv4(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    ip.is_private()
        || ip.is_link_local()
        || ip.is_loopback()
        || (o[0] == 100 && (64..=127).contains(&o[1]))
}

/// `https://konak[:port]` biçiminde ve konak özel/yerel mi? (`http://` kabul edilmez.)
pub fn duyuru_gecerli(taban: &str) -> bool {
    let Some(geri) = taban.trim().strip_prefix("https://") else {
        return false;
    };
    if geri.is_empty() || geri.contains(['/', '?', '#', '@', ' ']) {
        return false;
    }
    let konak = match geri.rsplit_once(':') {
        Some((k, p)) if !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()) => k,
        Some(_) => return false,
        None => geri,
    };
    if konak.is_empty() || konak.starts_with('[') {
        return false; // IPv6 şimdilik yok
    }
    if let Ok(ip) = konak.parse::<Ipv4Addr>() {
        return ozel_ipv4(ip);
    }
    let k = konak.to_ascii_lowercase();
    if !k
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'.'))
        || k.starts_with(['-', '.'])
        || k.ends_with(['-', '.'])
    {
        return false;
    }
    !k.contains('.') || k.ends_with(".local") || k.ends_with(".ts.net")
}

/// Duyurulan adresleri listeye uygular: eski `oto-*` yollar silinir, geçerli adresler (tekrarsız, en çok
/// `OTO_EN_COK`) `oto-1..` olarak başa konur. Liste değiştiyse true.
pub fn duyuru_birlestir(y: &mut Yollar, adresler: &[String]) -> Result<bool, YolHata> {
    let onceki = y.clone();
    y.yollar.retain(|v| !v.ad.starts_with(OTO_ON_EK));
    let mut yeni: Vec<Yol> = Vec::new();
    for a in adresler {
        if yeni.len() >= OTO_EN_COK {
            break;
        }
        let t = a.trim().trim_end_matches('/').to_string();
        if !duyuru_gecerli(&t) || yeni.iter().any(|v| v.taban == t) {
            continue;
        }
        // Elle eklenmiş aynı adresli yol varsa çift kayıt açma.
        if y.yollar.iter().any(|v| v.taban == t) {
            continue;
        }
        yeni.push(Yol::yeni(
            &format!("{OTO_ON_EK}{}", yeni.len() + 1),
            &t,
            None,
        )?);
    }
    let mut sonuc = yeni;
    sonuc.append(&mut y.yollar);
    y.yollar = sonuc;
    Ok(*y != onceki)
}

/// Yanıt gövdesinden adresleri çıkarır (`adresler` dizisi; metin olmayanlar atılır).
pub fn adresleri_coz(v: &Value) -> Vec<String> {
    v.get("adresler")
        .and_then(Value::as_array)
        .map(|d| {
            d.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// `g` (seçilmiş yola bağlı gönderici) ile sunucudan adresleri alır ve dosyayı günceller.
/// Dönüş: (değişti mi, uygulanan oto yol sayısı). Sunucu ucu yoksa (404) liste değişmez, hata dönmez.
pub fn guncelle(kok: &std::path::Path, g: &dyn Gonderici) -> Result<(bool, usize), YolHata> {
    let mut y = crate::yollar_oku(kok)?.ok_or(YolHata::HedefYok)?;
    let yanit = match g.al(BILGI_UCU) {
        Ok(v) => v,
        Err(GonderHata::Reddedildi { .. }) => return Ok((false, 0)),
        Err(e) => return Err(YolHata::Dosya(format!("yol bilgisi alınamadı: {e}"))),
    };
    let adresler = adresleri_coz(&yanit);
    let degisti = duyuru_birlestir(&mut y, &adresler)?;
    if degisti {
        crate::yollar_yaz(kok, &y)?;
    }
    let n = y
        .yollar
        .iter()
        .filter(|v| v.ad.starts_with(OTO_ON_EK))
        .count();
    Ok((degisti, n))
}
