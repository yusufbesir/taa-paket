# Sahip: Yusuf. yazan: [Claude#ortak-veri-havuzu 2026-11-12]
import copy
import sys
import tomllib
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import havuz_denetle as hd

KAT = tomllib.loads((Path(__file__).resolve().parent.parent / "docs" / "havuz-turleri.toml").read_text(encoding="utf-8"))


class Katalog(unittest.TestCase):
    def test_gercek_katalog_temiz(self):
        self.assertEqual(hd.katalog_denetle(KAT), [])

    def test_tablo_turu_iki_yazar(self):
        k = copy.deepcopy(KAT)
        k["turler"]["urun.liste"]["yazanlar"] = ["tutaa", "taarkod"]
        self.assertTrue(any("tek yazarı" in x for x in hd.katalog_denetle(k)))

    def test_bilinmeyen_program(self):
        k = copy.deepcopy(KAT)
        k["turler"]["urun.liste"]["okuyanlar"].append("yok-program")
        self.assertTrue(any("yok-program" in x for x in hd.katalog_denetle(k)))

    def test_kip_gecersiz(self):
        k = copy.deepcopy(KAT)
        k["turler"]["urun.liste"]["kip"] = "x"
        self.assertTrue(any("kip" in x for x in hd.katalog_denetle(k)))


class Program(unittest.TestCase):
    def test_gecerli(self):
        m = {"ad": "fiyatetiketi", "havuz_okur": ["urun.liste@1", "fiyat.liste@1"]}
        self.assertEqual(hd.program_denetle(m, KAT, "p"), [])

    def test_okuyan_degil(self):
        m = {"ad": "taadon", "havuz_okur": ["urun.liste@1"]}
        self.assertTrue(any("okuyanlar listesinde değil" in x for x in hd.program_denetle(m, KAT, "p")))

    def test_yazar_degil(self):
        m = {"ad": "taarkod", "havuz_yazar": ["urun.liste@1"]}
        self.assertTrue(any("yazanlar listesinde değil" in x for x in hd.program_denetle(m, KAT, "p")))

    def test_katalogda_yok_ve_surum(self):
        m = {"ad": "tutaa", "havuz_okur": ["yok.tur@1", "urun.liste@2"]}
        h = hd.program_denetle(m, KAT, "p")
        self.assertTrue(any("katalogda yok" in x for x in h))
        self.assertTrue(any("yüksek" in x for x in h))

    def test_ilgi_ailesi(self):
        self.assertEqual(hd.program_denetle({"ad": "taaegitim", "havuz_ilgi": ["egitim"]}, KAT, "p"), [])
        self.assertTrue(hd.program_denetle({"ad": "taaegitim", "havuz_ilgi": ["yok"]}, KAT, "p"))


if __name__ == "__main__":
    unittest.main()
