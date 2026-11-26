# Sahip: Yusuf. yazan: [Claude#taa-kasa-KS10 2026-11-26]
"""kasa_ciro_karsilastir testleri: satır adıyla eşleştirme, firma havale satırı, masraf toplamı, satır kayması."""
import os
import sys
import tempfile
import unittest
import zipfile

sys.path.insert(0, os.path.dirname(__file__))
import kasa_ciro_karsilastir as k  # noqa: E402


def sutun(i):
    return "ABCDEFGHIJ"[i]


def xlsx(yol, sayfalar, ortak=False):
    """Küçük xlsx yazar. sayfalar: {ad: [[değer, ...], ...]}; ortak=True: metinler sharedStrings'te."""
    dizgiler = []
    with zipfile.ZipFile(yol, "w") as z:
        wb = ['<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" '
              'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>']
        rels = ['<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">']
        for n, (ad, satirlar) in enumerate(sayfalar.items(), 1):
            wb.append(f'<sheet name="{ad}" sheetId="{n}" r:id="rId{n}"/>')
            rels.append(f'<Relationship Id="rId{n}" Type="x" Target="worksheets/sheet{n}.xml"/>')
            x = ['<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>']
            for r, satir in enumerate(satirlar, 1):
                x.append(f'<row r="{r}">')
                for c, v in enumerate(satir):
                    ref = f"{sutun(c)}{r}"
                    if v is None:
                        continue
                    if isinstance(v, str):
                        if ortak:
                            dizgiler.append(v)
                            x.append(f'<c r="{ref}" t="s"><v>{len(dizgiler) - 1}</v></c>')
                        else:
                            x.append(f'<c r="{ref}" t="inlineStr"><is><t>{v}</t></is></c>')
                    else:
                        x.append(f'<c r="{ref}"><f>1+1</f><v>{v}</v></c>')
                x.append("</row>")
            x.append("</sheetData></worksheet>")
            z.writestr(f"xl/worksheets/sheet{n}.xml", "".join(x))
        wb.append("</sheets></workbook>")
        rels.append("</Relationships>")
        z.writestr("xl/workbook.xml", "".join(wb))
        z.writestr("xl/_rels/workbook.xml.rels", "".join(rels))
        if ortak:
            si = "".join(f"<si><t>{s}</t></si>" for s in dizgiler)
            z.writestr("xl/sharedStrings.xml",
                       f'<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">{si}</sst>')


BAS = ["İŞLETME", "OUTLET MAĞAZA", "ANA MAĞAZA", "TOPLAM DÖVİZ", "01.11.2026", None, "NASIL DOLDURULUR?"]


def sayfa(nakit_outlet=100.0, havale_adi="FİRMA A", masraflar=(10.0, 5.0), bos_satir=False):
    s = [BAS, ["NAKİT TL", nakit_outlet, 200.0, None, nakit_outlet + 200.0]]
    if bos_satir:
        s.append([None])
    s += [
        ["BANKAYA HAVALE", None, None, None, "TUTAR"],
        [havale_adi, 50.0, 0.0, None, 50.0],
        ["MASRAFLAR"],
        ["KASA AÇIĞI", 0.0, 0.0, None, 0.0],
    ]
    s += [[f"masraf {i}", None, None, None, m] for i, m in enumerate(masraflar)]
    s += [["TOPLAM MASRAF", None, None, None, sum(masraflar)], ["GENEL TOPLAM", None, None, None, 999.0],
          ["GENEL TOPLAM", None, None, None, 1000.0]]
    return s


class Testler(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()

    def yol(self, ad):
        return os.path.join(self.d, ad)

    def test_ayni_dosyalar_farksiz(self):
        xlsx(self.yol("a.xlsx"), {"01.11.2026": sayfa()})
        xlsx(self.yol("b.xlsx"), {"01.11.2026": sayfa()}, ortak=True)
        r = k.karsilastir(self.yol("a.xlsx"), self.yol("b.xlsx"))
        self.assertEqual(r["sayfa"], 1)
        self.assertEqual(r["farklar"], [])

    def test_fark_satir_adiyla_bulunur_kayma_ve_firma_adi_onemsiz(self):
        xlsx(self.yol("a.xlsx"), {"01.11.2026": sayfa(), "02.11.2026": sayfa()})
        # Program çıktısında bir satır kaymış, firma adı farklı, masraflar başka sırada, nakit farklı.
        xlsx(self.yol("b.xlsx"), {"01.11.2026": sayfa(nakit_outlet=90.0, havale_adi="BAŞKA FİRMA",
                                                      masraflar=(5.0, 10.0), bos_satir=True)})
        r = k.karsilastir(self.yol("a.xlsx"), self.yol("b.xlsx"))
        anahtar = {(x["satir"], x["sutun"]): x["fark"] for x in r["farklar"]}
        self.assertEqual(anahtar, {("NAKİT TL", "MAĞAZA 1"): -10.0, ("NAKİT TL", "TOPLAM"): -10.0})
        self.assertEqual(r["yalniz_dolu"], ["02.11.2026"])

    def test_masraf_toplami_ve_ikinci_genel_toplam(self):
        xlsx(self.yol("a.xlsx"), {"01.11.2026": sayfa(masraflar=(10.0, 5.0))})
        b = sayfa(masraflar=(10.0, 6.0))
        b[-1][4] = 1001.0
        xlsx(self.yol("b.xlsx"), {"01.11.2026": b})
        r = k.karsilastir(self.yol("a.xlsx"), self.yol("b.xlsx"))
        satirlar = sorted(x["satir"] for x in r["farklar"])
        self.assertEqual(satirlar, ["GENEL TOPLAM (2)", "MASRAF SATIRLARI TOPLAMI", "TOPLAM MASRAF"])

    def test_cikis_kodu(self):
        xlsx(self.yol("a.xlsx"), {"01.11.2026": sayfa()})
        xlsx(self.yol("b.xlsx"), {"01.11.2026": sayfa(nakit_outlet=1.0)})
        self.assertEqual(k.main([self.yol("a.xlsx"), self.yol("a.xlsx")]), 0)
        self.assertEqual(k.main([self.yol("a.xlsx"), self.yol("b.xlsx"), "--json"]), 1)
        self.assertEqual(k.main([self.yol("yok.xlsx"), self.yol("b.xlsx")]), 2)


if __name__ == "__main__":
    unittest.main()
