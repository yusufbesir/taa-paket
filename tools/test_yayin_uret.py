# Sahip: Yusuf. yazan: [Claude#taa-paket-T4 2026-11-05]
"""yayin_uret.py testleri: geçici klasörde sahte paket zip'leri."""
import hashlib
import io
import json
import sys
import tempfile
import unittest
import zipfile
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import yayin_uret as yu  # noqa: E402


class Testler(unittest.TestCase):
    def setUp(self):
        self._t = tempfile.TemporaryDirectory()
        self.kok = Path(self._t.name)

    def tearDown(self):
        self._t.cleanup()

    def zip(self, ad="taa-x", surum="1.2.0", imza=True, paket=True, dosya=None) -> Path:
        yol = self.kok / (dosya or f"{ad}-{surum}.zip")
        p = {"sema": 1, "ad": ad, "surum": surum, "arayuz": "1", "gerektirir": {"taa-b": "1", "taa-a": "2"}}
        if imza:
            p["imza"] = {"alg": "ed25519", "anahtar_kimlik": "k", "deger": "00"}
        with zipfile.ZipFile(yol, "w") as z:
            if paket:
                z.writestr("paket.json", json.dumps(p))
            z.writestr("bin/x.txt", surum)
        return yol

    def test_tek_zip_yayin_uretir(self):
        z = self.zip()
        cikti = self.kok / "y" / "yayin.json"
        y = yu.uret([z], cikti)
        self.assertEqual(y["sema"], 1)
        p = y["paketler"][0]
        self.assertEqual((p["ad"], p["surum"], p["kanal"], p["arayuz"]), ("taa-x", "1.2.0", "kararli", "1"))
        self.assertEqual(list(p["gerektirir"]), ["taa-a", "taa-b"])
        self.assertEqual(p["tam"]["url"], "taa-x-1.2.0.zip")
        self.assertEqual(p["tam"]["sha256"], hashlib.sha256(z.read_bytes()).hexdigest())
        self.assertEqual(p["tam"]["boyut"], z.stat().st_size)
        self.assertEqual(p["yamalar"], [])
        self.assertNotIn("imza", y)
        self.assertEqual(json.loads(cikti.read_text(encoding="utf-8")), y)

    def test_taban_url_ve_en_az_surum(self):
        z = self.zip()
        y = yu.uret([z], self.kok / "yayin.json", taban_url="https://x/y/", en_az={"taa-x": "1.1.0"})
        self.assertEqual(y["paketler"][0]["tam"]["url"], "https://x/y/taa-x-1.2.0.zip")
        self.assertEqual(y["paketler"][0]["en_az_surum"], "1.1.0")
        with self.assertRaises(yu.YayinHata):
            yu.uret([z], self.kok / "y2.json", en_az={"taa-x": "1.3.0"})
        with self.assertRaises(yu.YayinHata):
            yu.uret([z], self.kok / "y3.json", taban_url="ftp://x")

    def test_onceki_ile_birlestirme_ve_yedek(self):
        cikti = self.kok / "yayin.json"
        z1 = self.zip(surum="1.0.0")
        z_b = self.zip(ad="taa-b", surum="2.0.0")
        yu.uret([z1, z_b], cikti)
        z2 = self.zip(surum="1.1.0")
        z1y = self.zip(surum="1.0.0", dosya="taa-x-1.0.0-yeniden.zip")
        uyarilar = []
        y = yu.uret([z2, z1y], cikti, onceki=cikti, uyar=uyarilar.append)
        adlar = [(p["ad"], p["surum"]) for p in y["paketler"]]
        self.assertEqual(adlar, [("taa-b", "2.0.0"), ("taa-x", "1.0.0"), ("taa-x", "1.1.0")])
        # aynı sürüm yenisiyle değişti
        self.assertEqual(y["paketler"][1]["tam"]["url"], "taa-x-1.0.0-yeniden.zip")
        self.assertTrue((self.kok / "yayin.json.once-1").exists())
        self.assertTrue(any("ezilmedi" in u for u in uyarilar))
        # sıra sayısal: 1.10.0 > 1.9.0
        y = yu.uret([self.zip(surum="1.10.0"), self.zip(surum="1.9.0")], self.kok / "s.json")
        self.assertEqual([p["surum"] for p in y["paketler"]], ["1.9.0", "1.10.0"])

    def test_hatalar(self):
        with self.assertRaises(yu.YayinHata):
            yu.uret([self.zip(paket=False)], self.kok / "a.json")
        bozuk = self.kok / "bozuk.zip"
        bozuk.write_bytes(b"zip degil")
        with self.assertRaises(yu.YayinHata):
            yu.uret([bozuk], self.kok / "b.json")
        with self.assertRaises(yu.YayinHata):
            yu.uret([self.zip(ad="Kötü")], self.kok / "c.json")
        with self.assertRaises(yu.YayinHata):
            yu.uret([self.zip(), self.zip(dosya="ayni.zip")], self.kok / "d.json")
        with self.assertRaises(yu.YayinHata):
            yu.uret([self.zip()], self.kok / "e.json", kanal="Kararlı")
        with self.assertRaises(yu.YayinHata):
            yu.uret([], self.kok / "f.json")
        with self.assertRaises(yu.YayinHata):
            yu.uret([self.zip()], self.kok / "g.json", onceki=self.zip())

    def test_imzasiz_uyari_ve_zorunlu(self):
        uyarilar = []
        yu.uret([self.zip(imza=False)], self.kok / "a.json", uyar=uyarilar.append)
        self.assertTrue(any("imzasız" in u for u in uyarilar))
        with self.assertRaises(yu.YayinHata):
            yu.uret([self.zip(imza=False)], self.kok / "b.json", imzali_zorunlu=True)

    def test_main(self):
        z = self.zip()
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            kod = yu.main([str(z), "--cikti", str(self.kok / "m.json"), "--en-az-surum", "taa-x=1.0.0", "--kanal", "deneme"])
        self.assertEqual(kod, 0, err.getvalue())
        self.assertIn("taa-imza imzala", out.getvalue())
        y = json.loads((self.kok / "m.json").read_text(encoding="utf-8"))
        self.assertEqual((y["paketler"][0]["kanal"], y["paketler"][0]["en_az_surum"]), ("deneme", "1.0.0"))
        with redirect_stdout(out), redirect_stderr(err):
            kod = yu.main([str(self.kok / "yok.zip"), "--cikti", str(self.kok / "n.json")])
        self.assertEqual(kod, 2)
        self.assertIn("Hata:", err.getvalue())


if __name__ == "__main__":
    unittest.main()
