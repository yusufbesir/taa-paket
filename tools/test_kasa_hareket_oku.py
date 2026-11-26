# Sahip: Yusuf. yazan: [Claude#taa-kasa-KS7 2026-11-26]
"""kasa_hareket_oku testleri: anonim, uydurma tutarlarla (gerçek ciro, ad ya da firma yok)."""
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import kasa_hareket_oku as k  # noqa: E402

# pdftotext -raw biçiminde uydurma hareket özeti (bölünmüş tutar, çok satırlı banka adı ve başlık dahil).
HAREKET = """Mağaza Hareket Özeti
9-9-9-9 Deneme Mağaza
25.11.2026
Satış
Süreç Açıklaması İade Miktar Tutar (VD)
Fatura Peşin Satış Normal 10,00 1.000,00 0,00 900,00 100,00 11.200,00
Fatura Peşin Satış İade -1,00 -100,00 0,00 -90,00 -10,00 -500,00
Peşin Satış Toplam 9,00 900,00 0,00 810,00 90,00 10.700,00
Genel Toplam 9,00 900,00 0,00 810,00 90,00 10.700,00
Ödemeler
Ödeme Tipi
Taksitli
Satış
Peşinatı
Diğer
Ödemeler
Toplam
Nakit TRY 0 0,00 3.000,00 0,00 0,00 0,00
-
2.850,0
0
150,00
Nakit USD 0 0,00 1.000,00 0,00 0,00 0,00 -1.000,00 0,00
Kredi Kartı TRY
ÖRNEK
BANKA A.S.
0 0,00 4.000,00 0,00 0,00 0,00 0,00 4.000,00
Kredi Kartı TRY DENEME BANKASI 0 0,00 1.000,00 0,00 0,00 0,00 0,00 1.000,00
Banka
Havalesi / EFT
TRY 0 0,00 1.250,00 0,00 0,00 0,00 0,00 1.250,00
Para Üstü TRY 0 0,00 0,00 0,00 0,00 -50,00 0,00 -50,00
Kredi Çeki TRY 0 0,00 1.000,00 0,00 0,00 -1.000,00 0,00 0,00
Genel Toplam 0,00 10.700,00 0,00 0,00 -1.050,00 -3.850,00 5.800,00
Masraflar
Masraf Kodu Masraf Adı Miktar Vergi Matrahı Vergi Net Tutar
E-001 Deneme gideri -1,00 -90,00 -10,00 -100,00
Genel Toplam -1,00 -90,00 -10,00 -100,00
Önceki Günden Devir 0,00
Nakit Kasa Yekünü 0,00
"""


class HareketTesti(unittest.TestCase):
    def test_bolum_satis_odeme_masraf(self):
        r = k.hareket_coz(HAREKET)
        self.assertEqual(len(r["bolumler"]), 1, "başlıktaki 'Satış' yeni bölüm açmaz")
        b = r["bolumler"][0]
        self.assertEqual(b["satis"], {"normal": 1120000, "iade": -50000, "toplam": 1070000})
        self.assertEqual(b["masraf"], -10000)
        self.assertEqual(len(b["odemeler"]), 7, "bölüm sonrası 'Nakit Kasa' satırları ödeme değildir")
        nakit = b["odemeler"][0]
        self.assertEqual((nakit["pesinat"], nakit["diger"]), (300000, -285000), "bölünmüş tutar birleşir")
        self.assertEqual(b["odemeler"][2]["banka"], "ÖRNEK BANKA A.S.")

    def test_sablon_ozeti_ve_uclu(self):
        b = k.hareket_coz(HAREKET)["bolumler"][0]
        o = k.sablon_ozeti(b)
        # NAKİT TL = nakit peşinat + para üstü − masraf
        self.assertEqual(o["nakit_tl"], 300000 - 5000 - 10000)
        self.assertEqual(o["kredi_karti"], 500000)
        self.assertEqual(o["havale_ve_iyzico_odemesi"], 125000)
        self.assertEqual(o["degisim"], 100000)
        self.assertEqual(o["doviz_tl"], {"USD": 100000})
        # Genel toplam 10.700 = 2.850 + 1.000 + 5.000 + 1.250 + 100 masraf + 500 internet değişimi;
        # Z 11.200 = 10.700 + 1.000 değişim − 500 internet değişimi
        r = k.uclu({"bolumler": [b]}, [1_120_000], 1_070_000, internet_degisimi=50_000, iade=0)
        farklar = {x["ad"]: x["fark"] for x in r}
        self.assertTrue(all(v == 0 for v in farklar.values()), farklar)
        r = k.uclu({"bolumler": [b]}, [1_119_000], 1_070_000, internet_degisimi=50_000)
        self.assertEqual(r[0]["fark"], -1000)

    def test_tanınmayan_metin(self):
        with self.assertRaises(ValueError):
            k.hareket_coz("boş belge")


class OdemeListesiTesti(unittest.TestCase):
    def satirlar(self):
        bas = {2: "Nakit Kasa Hesap Kodu", 15: "Adı-Soyadı", 12: "Perakende Müşteri Kodu", 19: "Tipi",
               22: "Ödeme", 23: "Ödeme (D)", 27: "Kasiyer"}
        r = lambda tip, od, kas, ad="", mus="", odd=0: {19: tip, 22: od, 23: odd, 27: kas, 15: ad, 12: mus}
        return [
            bas,
            {2: "Döküman Para Birimi: TRY "},
            r("Devir", 999, ""),
            r("Peşin Satış", 1000, "A", "Kişi Bir", "1-4-1"),
            r("Peşin Satış - İade", -100, "A", "Kasa", ""),          # müşterisiz: masraf
            r("Peşin Satış - İade", -50, "A", "Kişi İki", "1-4-2"),  # müşterili: para üstü
            r("Ödeme - İade", -850, "A", "Elden Teslim"),
            r("Peşin Satış", 300, "B", "Kişi Üç", "1-4-3"),
            r("Ödeme - İade", -300, "B", "Elden Teslim"),
            {2: "Döküman Para Birimi: USD "},
            r("Peşin Satış", 420, "A", "Kişi Bir", "1-4-1", odd=10),
            r("Ödeme - İade", -420, "A", "Elden Teslim", odd=-10),
        ]

    def test_magaza_ve_tur_toplamlari_ad_yazilmaz(self):
        s = k.odeme_listesi_coz(self.satirlar())
        self.assertEqual(set(s), {"mağaza 1", "mağaza 2"})
        a = s["mağaza 1"]["TRY"]
        self.assertEqual(a, {"satis": 100000, "masraf": -10000, "para_ustu": -5000, "elden_teslim": -85000})
        # Elden teslim = satış + para üstü − masraf (şablon NAKİT TL)
        self.assertEqual(-a["elden_teslim"], a["satis"] + a["para_ustu"] + a["masraf"])
        self.assertEqual(s["mağaza 1"]["USD"]["satis_doviz"], 1000)
        self.assertNotIn("Kişi", repr(s))
        self.assertIn("A", k.odeme_listesi_coz(self.satirlar(), magaza_adi=True))


if __name__ == "__main__":
    unittest.main()
