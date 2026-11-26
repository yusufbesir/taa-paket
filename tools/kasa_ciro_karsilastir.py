#!/usr/bin/env python3
# Sahip: Yusuf. yazan: [Claude#taa-kasa-KS10 2026-11-26]
"""Akşam ciro Excel'i karşılaştırıcı (TAA Kasa).

Şubenin elle doldurduğu ciro Excel'ini ("KASIM AYI CİRO ŞABLONU" düzeni) TAA Kasa'nın aynı ay için ürettiği
ciro Excel'iyle (Kayıtlar ve Excel → "Akşam ciro Excel'i") karşılaştırır ve farkları listeler.

    python3 tools/kasa_ciro_karsilastir.py dolu.xlsx program.xlsx [--gun 26.11.2026] [--json]

Eşleştirme hücre adresiyle değil, satır adıyla yapılır (A sütunu) ve sütun konumuyla (B, C … mağaza sütunları,
TOPLAM DÖVİZ, toplam). Böylece satır kayması olsa da aynı satır bulunur. Özel durumlar:
- "BANKAYA HAVALE" bölümündeki ilk satır (firma hesabı; adı şubeden şubeye değişir) "HAVALE (FİRMA)" sayılır.
- Masraf satırları serbest metindir; tek tek değil, "TOPLAM MASRAF" ve satır toplamları karşılaştırılır.
- Döviz kurları satırında sütunlar döviz başlıklarına (DOLAR, EURO …) göre adlandırılır.

Değerler dosyalarda kayıtlı (önbellek) değerlerdir; Excel'de kaydedilmiş dosyalarda formül sonuçları vardır.
Yalnız standart kitaplık kullanılır. Dosya içeriği hiçbir yere gönderilmez. Çıkış kodu: fark yoksa 0, varsa 1,
hata 2.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import zipfile
import xml.etree.ElementTree as ET

NS = {
    "m": "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
    "r": "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
    "pr": "http://schemas.openxmlformats.org/package/2006/relationships",
}
TOLERANS = 0.005


def buyuk(s: str) -> str:
    s = (s or "").replace("i", "İ").replace("ı", "I").upper()
    return re.sub(r"\s+", " ", s).strip()


def sutun_no(ref: str) -> int:
    n = 0
    for ch in re.match(r"[A-Z]+", ref).group(0):
        n = n * 26 + ord(ch) - 64
    return n


def oku(yol: str) -> dict[str, dict[tuple[int, int], object]]:
    """xlsx → {sayfa adı: {(satır, sütun): değer}} (değer: float ya da str)."""
    with zipfile.ZipFile(yol) as z:
        adlar = set(z.namelist())
        ortak: list[str] = []
        if "xl/sharedStrings.xml" in adlar:
            for si in ET.fromstring(z.read("xl/sharedStrings.xml")).findall("m:si", NS):
                ortak.append("".join(t.text or "" for t in si.iter(f"{{{NS['m']}}}t")))
        wb = ET.fromstring(z.read("xl/workbook.xml"))
        rels = ET.fromstring(z.read("xl/_rels/workbook.xml.rels"))
        hedef = {r.get("Id"): r.get("Target") for r in rels.findall("pr:Relationship", NS)}
        sonuc = {}
        for s in wb.find("m:sheets", NS).findall("m:sheet", NS):
            t = hedef[s.get(f"{{{NS['r']}}}id")].lstrip("/")
            yol_ic = t if t.startswith("xl/") else "xl/" + t
            hucreler: dict[tuple[int, int], object] = {}
            for c in ET.fromstring(z.read(yol_ic)).iter(f"{{{NS['m']}}}c"):
                ref = c.get("r")
                satir = int(re.search(r"\d+", ref).group(0))
                tur = c.get("t")
                v = c.find("m:v", NS)
                if tur == "inlineStr":
                    deger = "".join(x.text or "" for x in c.iter(f"{{{NS['m']}}}t"))
                elif tur == "s" and v is not None:
                    deger = ortak[int(v.text)]
                elif tur in ("str", "e") and v is not None:
                    deger = v.text or ""
                elif v is not None and v.text not in (None, ""):
                    try:
                        deger = float(v.text)
                    except ValueError:
                        deger = v.text
                else:
                    continue
                hucreler[(satir, sutun_no(ref))] = deger
            sonuc[s.get("name")] = hucreler
        return sonuc


def anahtarlar(h: dict[tuple[int, int], object]) -> dict[tuple[str, str], float]:
    """Bir sayfanın sayısal değerlerini (satır adı, sütun adı) anahtarına çevirir."""
    if not h:
        return {}
    # Tablo genişliği: 1. satırda dolu son sütun (G'deki yardım sütunu hariç: araya boş F girer).
    bas = [c for (r, c) in h if r == 1 and c > 1]
    son = 1
    for c in sorted(bas):
        if c == son + 1:
            son = c
    magaza = max(son - 3, 0)  # A + mağazalar + TOPLAM DÖVİZ + toplam
    adlar = {}
    for c in range(2, son + 1):
        if c <= 1 + magaza:
            adlar[c] = f"MAĞAZA {c - 1}"
        elif c == son - 1:
            adlar[c] = "TOPLAM DÖVİZ"
        else:
            adlar[c] = "TOPLAM"
    satir_adlari: dict[int, str] = {}
    bolum = ""
    masraf = 0
    doviz_adlari = None
    for r in sorted({r for (r, _) in h}):
        a = h.get((r, 1))
        a = buyuk(a) if isinstance(a, str) else ""
        b = h.get((r, 2))
        if isinstance(b, str) and buyuk(b) == "DOLAR":
            doviz_adlari = {c: buyuk(h.get((r, c)) or "") for c in range(2, 6)}
            continue
        if a == "BANKAYA HAVALE":
            bolum = "havale"
            continue
        if a == "MASRAFLAR":
            bolum = "masraf"
            continue
        if bolum == "havale":
            a = "HAVALE (FİRMA)"
            bolum = ""
        elif bolum == "masraf" and a not in ("KASA AÇIĞI", "TOPLAM MASRAF"):
            masraf += 1
            a = f"MASRAF SATIRI {masraf}"
        if a == "TOPLAM MASRAF":
            bolum = ""
        if not a:
            continue
        # Aynı ad iki kez geçebilir (GENEL TOPLAM): sırayla numaralanır.
        while a in satir_adlari.values():
            m = re.match(r"(.*) \((\d+)\)$", a)
            a = f"{m.group(1)} ({int(m.group(2)) + 1})" if m else f"{a} (2)"
        satir_adlari[r] = a
    sonuc = {}
    for (r, c), v in h.items():
        if r not in satir_adlari or c < 2 or c > max(son, 5) or not isinstance(v, float):
            continue
        ad = satir_adlari[r]
        if ad.startswith("GÜNLÜK DÖVİZ KURLARI") or "DÖVİZ KURU" in ad:
            sut = (doviz_adlari or {}).get(c, f"SÜTUN {c}")
        else:
            sut = adlar.get(c)
            if sut is None:
                continue
        sonuc[(ad, sut)] = v
    return sonuc


def masraf_satirlari(k: dict[tuple[str, str], float]) -> dict[tuple[str, str], float]:
    """Masraf satırları sırasız karşılaştırılır: satır başına değil, sütun toplamı olarak."""
    toplam: dict[tuple[str, str], float] = {}
    for (ad, sut), v in list(k.items()):
        if ad.startswith("MASRAF SATIRI"):
            toplam[("MASRAF SATIRLARI TOPLAMI", sut)] = toplam.get(("MASRAF SATIRLARI TOPLAMI", sut), 0.0) + v
            del k[(ad, sut)]
    k.update(toplam)
    return k


def karsilastir(dolu: str, program: str, gun: str | None = None) -> dict:
    a, b = oku(dolu), oku(program)
    sayfalar = [s for s in a if s in b and (gun is None or s == gun)]
    farklar = []
    for s in sayfalar:
        ka = masraf_satirlari(anahtarlar(a[s]))
        kb = masraf_satirlari(anahtarlar(b[s]))
        for k in sorted(set(ka) | set(kb)):
            va, vb = ka.get(k, 0.0), kb.get(k, 0.0)
            if abs(va - vb) > TOLERANS:
                farklar.append(
                    {"sayfa": s, "satir": k[0], "sutun": k[1], "dolu": round(va, 2), "program": round(vb, 2),
                     "fark": round(vb - va, 2)}
                )
    return {
        "sayfa": len(sayfalar),
        "yalniz_dolu": sorted(set(a) - set(b)),
        "yalniz_program": sorted(set(b) - set(a)),
        "farklar": farklar,
    }


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description="Akşam ciro Excel'ini TAA Kasa çıktısıyla karşılaştırır.")
    p.add_argument("dolu", help="şubenin doldurduğu ciro Excel'i (.xlsx)")
    p.add_argument("program", help="TAA Kasa'nın ürettiği ciro Excel'i (.xlsx)")
    p.add_argument("--gun", help="yalnız bu sayfa (ör. 26.11.2026)")
    p.add_argument("--json", action="store_true", help="sonucu JSON yaz")
    ns = p.parse_args(argv)
    try:
        r = karsilastir(ns.dolu, ns.program, ns.gun)
    except (OSError, KeyError, zipfile.BadZipFile, ET.ParseError) as e:
        print(f"okunamadı: {e}", file=sys.stderr)
        return 2
    if ns.json:
        print(json.dumps(r, ensure_ascii=False, indent=1))
    else:
        print(f"{r['sayfa']} sayfa karşılaştırıldı, {len(r['farklar'])} fark.")
        for x in r["farklar"]:
            print(f"{x['sayfa']}  {x['satir']}  [{x['sutun']}]  dolu={x['dolu']:.2f}  program={x['program']:.2f}"
                  f"  fark={x['fark']:+.2f}")
        if r["yalniz_dolu"]:
            print("Programda olmayan sayfalar:", ", ".join(r["yalniz_dolu"]))
    return 1 if r["farklar"] else 0


if __name__ == "__main__":
    sys.exit(main())
