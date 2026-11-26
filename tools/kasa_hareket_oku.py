#!/usr/bin/env python3
# Sahip: Yusuf. yazan: [Claude#taa-kasa-KS7 2026-11-26]
"""Nebim "Mağaza Hareket Özeti" (PDF) ve "Alınan Ödemeler Listesi" (xlsx) okuyucuları, üçlü karşılaştırma.

Yalnız yapı okunur; müşteri adı, kasiyer adı ve firma adı hiçbir çıktıya yazılmaz (kasiyer bölümleri
"kasiyer 1, 2…" diye numaralanır; ödeme listesindeki "Kasiyer" sütunu mağaza adıdır, o da yalnız
`--magaza-adi` ile istenirse yazılır, varsayılan "mağaza 1, 2…").

Üçlü karşılaştırma (25.11.2026 dolu dosyalarıyla kuruşu kuruşuna doğrulandı):
  1. Z raporları toplamı = Nebim "Fatura Peşin Satış Normal" net tutarı
  2. Nebim "Peşin Satış Toplam" net = ciro Excel'inin son GENEL TOPLAM'ı
  3. Z toplamı = GENEL TOPLAM + DEĞİŞİM − İNTERNET DEĞİŞİMİ (+ iade)   (TAA Kasa `z_beklenen`)
  DEĞİŞİM = hareket özetindeki "Kredi Çeki" peşinatı.
Ödeme tipleri şablon satırlarına:
  NAKİT TL      = Nakit TRY peşinat + Para Üstü − masraf      (= ödeme listesindeki "Elden Teslim")
  döviz         = Nakit <döviz> peşinat (TL karşılığı)
  KREDİ KARTI   = tüm Kredi Kartı satırları (banka = kartı veren banka; QR dahil) = ZİRAAT + İYZİCO (iki POS terminali; E-FATURA POS kaldırıldı)
  HAVALE        = Banka Havalesi / EFT = BANKAYA HAVALE + İYZİCO ÖDEMESİ
  masraf        = Masraflar genel toplamı
  İNTERNET DEĞİŞİMİ hareket özetinde ödeme olarak yoktur; Excel'den alınır.

Kullanım:
  kasa_hareket_oku.py hareket rapor.pdf [--json]
  kasa_hareket_oku.py odeme liste.xlsx [--json]
  kasa_hareket_oku.py uclu rapor.pdf --z 1.234,56 --z 99,00 --excel-genel-toplam 1.333,56 [--internet-degisimi 0]
Yalnız standart kütüphane (PDF için sistemdeki `pdftotext`).
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import zipfile
import xml.etree.ElementTree as ET

TUTAR = re.compile(r"-?\d{1,3}(?:\.\d{3})*,\d{2}|-?\d+,\d{2}")
ODEME_TIPLERI = ("Nakit", "Kredi Kartı", "Banka Havalesi / EFT", "Para Üstü", "Kredi Çeki")


def kurus(s: str) -> int:
    """'1.234,56' / '-12,34' -> kuruş."""
    s = s.strip()
    eksi = s.startswith("-")
    s = s.lstrip("-").replace(".", "").replace(",", ".")
    k = int(round(float(s) * 100))
    return -k if eksi else k


def tl(k: int) -> str:
    m = abs(k)
    g = f"{m // 100:,}".replace(",", ".")
    return f"{'-' if k < 0 else ''}{g},{m % 100:02d}"


def pdf_metni(yol: str) -> str:
    return subprocess.run(["pdftotext", "-raw", yol, "-"], check=True, capture_output=True, text=True).stdout


def _onar(metin: str) -> str:
    """pdftotext'in böldüğü tutarları birleştirir: '-\\n1.234,5\\n6' -> '-1.234,56'."""
    metin = re.sub(r"(^|\s)-\n(?=\d)", r"\1-", metin)
    metin = re.sub(r"(\d,\d)\n(\d)(?=\s|$)", r"\1\2", metin)
    return metin


def _satis(satirlar: list[str]) -> dict:
    s = {}
    for l in satirlar:
        t = TUTAR.findall(l)
        if l.startswith("Fatura Peşin Satış Normal") and t:
            s["normal"] = kurus(t[-1])
        elif l.startswith("Fatura Peşin Satış İade") and t:
            s["iade"] = kurus(t[-1])
        elif l.startswith("Peşin Satış Toplam") and t:
            s["toplam"] = kurus(t[-1])
    return s


def _odemeler(blok: str) -> list[dict]:
    """Ödemeler bölümü: her satır bir ödeme tipi + para birimi (+ banka). 7 tutar sütunu:
    taksitli satış peşinatı, peşinat, taksit ödemesi, taksit ödeme iadesi, peşinat iadesi, diğer, toplam."""
    blok = blok.replace("Banka\nHavalesi / EFT", "Banka Havalesi / EFT")
    desen = re.compile(r"^(" + "|".join(re.escape(t) for t in ODEME_TIPLERI) + r")\b", re.M)
    son = blok.find("Genel Toplam")
    if son < 0:
        son = len(blok)
    # "Nakit Kasa Yekünü" gibi bölüm sonrası satırlar ödeme satırı değildir: yalnız Genel Toplam'a kadar.
    yerler = [m.start() for m in desen.finditer(blok) if m.start() < son]
    v = []
    for i, b in enumerate(yerler):
        e = yerler[i + 1] if i + 1 < len(yerler) else son
        parca = blok[b:e]
        tip = desen.match(parca).group(1)
        kalan = parca[len(tip):]
        para = re.search(r"\b([A-Z]{3})\b", kalan)
        t = [kurus(x) for x in TUTAR.findall(kalan)]
        if len(t) < 7:
            continue
        t = t[-7:]
        banka = ""
        if tip == "Kredi Kartı" and para:
            # Banka adı: para biriminden sonra, ilk sayıya kadar (kart veren banka; kişi adı değildir).
            ara = kalan[para.end():]
            ara = re.split(r"\s\d", " " + ara.strip(), maxsplit=1)[0]
            banka = re.sub(r"\s+", " ", ara).strip()
        v.append({
            "tip": tip, "para": para.group(1) if para else "", "banka": banka,
            "pesinat": t[1], "pesinat_iadesi": t[4], "diger": t[5], "toplam": t[6],
        })
    return v


def _masraf(blok: str) -> int | None:
    for l in blok.splitlines():
        if l.startswith("Genel Toplam"):
            t = TUTAR.findall(l)
            if t:
                return kurus(t[-1])
    return None


def hareket_coz(metin: str) -> dict:
    """`pdftotext -raw` çıktısından hareket özeti. Kasiyer bazlı raporda her "Satış" başlığı bir bölümdür."""
    metin = _onar(metin)
    satirlar = metin.splitlines()
    bas = [i for i, l in enumerate(satirlar)
           if l.strip() == "Satış" and any(x.startswith("Süreç") for x in satirlar[i + 1:i + 3])]
    bolumler = []
    for n, b in enumerate(bas):
        e = bas[n + 1] if n + 1 < len(bas) else len(satirlar)
        parca = "\n".join(satirlar[b:e])
        o = parca.find("\nÖdemeler")
        m = parca.find("\nMasraflar")
        bolum = {"satis": _satis(satirlar[b:e])}
        if o >= 0:
            bolum["odemeler"] = _odemeler(parca[o:m if m > o else len(parca)])
        if m >= 0:
            bolum["masraf"] = _masraf(parca[m:])
        if bolum["satis"]:
            bolumler.append(bolum)
    if not bolumler:
        raise ValueError("hareket özeti tanınmadı (Satış bölümü yok)")
    return {"bolumler": bolumler, "genel": bolumler[0] if len(bolumler) == 1 else None}


def sablon_ozeti(b: dict) -> dict:
    """Bir hareket özeti bölümünü ciro şablonu satırlarına toplar (kuruş)."""
    od = b.get("odemeler", [])
    masraf = -(b.get("masraf") or 0)
    pesin = lambda tip, para=None: sum(x["pesinat"] for x in od if x["tip"] == tip and (para is None or x["para"] == para))
    para_ustu = sum(x["pesinat_iadesi"] for x in od if x["tip"] == "Para Üstü")
    doviz = {x["para"]: x["pesinat"] for x in od if x["tip"] == "Nakit" and x["para"] not in ("TRY", "")}
    kart = {x["banka"]: x["pesinat"] for x in od if x["tip"] == "Kredi Kartı"}
    return {
        "nakit_tl": pesin("Nakit", "TRY") + para_ustu - masraf,
        "doviz_tl": doviz,
        "kredi_karti": sum(kart.values()),
        "kredi_karti_banka": kart,
        "havale_ve_iyzico_odemesi": pesin("Banka Havalesi / EFT"),
        "degisim": pesin("Kredi Çeki"),
        "para_ustu": para_ustu,
        "masraf": masraf,
    }


# ---------------------------------------------------------------- ödeme listesi (xlsx)

NS = {"m": "http://schemas.openxmlformats.org/spreadsheetml/2006/main"}


def _xlsx_satirlar(yol: str) -> list[dict[int, object]]:
    with zipfile.ZipFile(yol) as z:
        ortak = []
        if "xl/sharedStrings.xml" in z.namelist():
            for si in ET.fromstring(z.read("xl/sharedStrings.xml")).findall("m:si", NS):
                ortak.append("".join(t.text or "" for t in si.iter("{%s}t" % NS["m"])))
        ad = sorted(n for n in z.namelist() if n.startswith("xl/worksheets/sheet"))[0]
        kok = ET.fromstring(z.read(ad))
    v = []
    for r in kok.iter("{%s}row" % NS["m"]):
        s: dict[int, object] = {}
        for c in r.findall("m:c", NS):
            ref = re.match(r"([A-Z]+)", c.get("r")).group(1)
            no = 0
            for ch in ref:
                no = no * 26 + ord(ch) - 64
            d = c.find("m:v", NS)
            is_ = c.find("m:is", NS)
            if c.get("t") == "s" and d is not None:
                s[no] = ortak[int(d.text)]
            elif c.get("t") == "inlineStr" and is_ is not None:
                s[no] = "".join(t.text or "" for t in is_.iter("{%s}t" % NS["m"]))
            elif d is not None and d.text is not None:
                try:
                    s[no] = float(d.text)
                except ValueError:
                    s[no] = d.text
        v.append(s)
    return v


def odeme_listesi_coz(satirlar: list[dict[int, object]], magaza_adi: bool = False) -> dict:
    """Alınan ödemeler listesi (nakit kasa hareketleri). Sütunlar başlık satırından bulunur.
    Satır türleri: Peşin Satış (satış), Peşin Satış - İade müşterili (para üstü/iade), müşterisiz (kasadan
    masraf), "Elden Teslim" (gün sonu teslim = şablon NAKİT TL), Devir (atlanır). Ad sütunu okunmaz."""
    bas = None
    for s in satirlar:
        d = {v.strip(): k for k, v in s.items() if isinstance(v, str)}
        if "Tipi" in d and "Ödeme" in d and "Kasiyer" in d:
            bas = d
            break
    if bas is None:
        raise ValueError("ödeme listesi başlığı bulunamadı")
    c_tip, c_od, c_odd, c_kas = bas["Tipi"], bas["Ödeme"], bas.get("Ödeme (D)"), bas["Kasiyer"]
    c_ad = bas.get("Adı-Soyadı")
    c_mus = bas.get("Perakende Müşteri Kodu")
    para = ""
    magazalar: dict[str, str] = {}
    sonuc: dict[str, dict[str, dict[str, int]]] = {}
    for s in satirlar:
        for v in s.values():
            if isinstance(v, str) and v.startswith("Döküman Para Birimi:"):
                para = v.split(":", 1)[1].strip()
        tip = s.get(c_tip)
        if not isinstance(tip, str) or tip in ("Devir", "Tipi"):
            continue
        kas = str(s.get(c_kas) or "").strip()
        etiket = magazalar.setdefault(kas, kas if magaza_adi else f"mağaza {len(magazalar) + 1}")
        tutar = int(round(float(s.get(c_od) or 0) * 100))
        doviz = int(round(float(s.get(c_odd) or 0) * 100)) if c_odd else 0
        ad = str(s.get(c_ad) or "").strip() if c_ad else ""
        musteri = bool(s.get(c_mus)) if c_mus else True
        if ad == "Elden Teslim":
            tur = "elden_teslim"
        elif tip == "Peşin Satış":
            tur = "satis"
        elif tip == "Peşin Satış - İade":
            tur = "para_ustu" if musteri else "masraf"
        else:
            tur = "diger"
        m = sonuc.setdefault(etiket, {}).setdefault(para, {})
        m[tur] = m.get(tur, 0) + tutar
        if para != "TRY" and doviz:
            m[tur + "_doviz"] = m.get(tur + "_doviz", 0) + doviz
    return sonuc


# ---------------------------------------------------------------- üçlü karşılaştırma

def uclu(hareket: dict, z: list[int], excel_genel_toplam: int, internet_degisimi: int = 0, iade: int = 0) -> list[dict]:
    b = hareket["bolumler"][0]
    s = b["satis"]
    o = sablon_ozeti(b)
    zt = sum(z)
    satir = lambda ad, a, bb: {"ad": ad, "beklenen": a, "bulunan": bb, "fark": bb - a}
    return [
        satir("Z toplamı ↔ Nebim Peşin Satış Normal", s.get("normal", 0), zt),
        satir("Nebim Peşin Satış Toplam ↔ Excel son GENEL TOPLAM", s.get("toplam", 0), excel_genel_toplam),
        satir("Z toplamı ↔ GENEL TOPLAM + DEĞİŞİM − İNTERNET DEĞİŞİMİ + iade",
              excel_genel_toplam + o["degisim"] - internet_degisimi + iade, zt),
        satir("Nebim ödemeler (nakit+döviz+kart+havale+masraf) + İNTERNET DEĞİŞİMİ ↔ GENEL TOPLAM",
              excel_genel_toplam,
              o["nakit_tl"] + sum(o["doviz_tl"].values()) + o["kredi_karti"] + o["havale_ve_iyzico_odemesi"]
              + o["masraf"] + internet_degisimi),
    ]


def _main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    alt = p.add_subparsers(dest="komut", required=True)
    h = alt.add_parser("hareket")
    h.add_argument("pdf")
    h.add_argument("--json", action="store_true")
    o = alt.add_parser("odeme")
    o.add_argument("xlsx")
    o.add_argument("--magaza-adi", action="store_true", help="mağaza adını (Kasiyer sütunu) yaz")
    u = alt.add_parser("uclu")
    u.add_argument("pdf")
    u.add_argument("--z", action="append", default=[], help="Z raporu toplamı (her yazarkasa için)")
    u.add_argument("--excel-genel-toplam", required=True)
    u.add_argument("--internet-degisimi", default="0,00")
    u.add_argument("--iade", default="0,00")
    a = p.parse_args(argv)
    if a.komut == "hareket":
        r = hareket_coz(pdf_metni(a.pdf))
        r["sablon"] = [sablon_ozeti(b) for b in r["bolumler"]]
        print(json.dumps(r, ensure_ascii=False, indent=1))
        return 0
    if a.komut == "odeme":
        print(json.dumps(odeme_listesi_coz(_xlsx_satirlar(a.xlsx), a.magaza_adi), ensure_ascii=False, indent=1))
        return 0
    r = uclu(hareket_coz(pdf_metni(a.pdf)), [kurus(x) for x in a.z], kurus(a.excel_genel_toplam),
             kurus(a.internet_degisimi), kurus(a.iade))
    for x in r:
        print(f"{'✓' if x['fark'] == 0 else '✕'} {x['ad']}: {tl(x['bulunan'])} / {tl(x['beklenen'])} fark {tl(x['fark'])}")
    return 1 if any(x["fark"] for x in r) else 0


if __name__ == "__main__":
    sys.exit(_main())
