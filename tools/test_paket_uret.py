# Sahip: Yusuf. yazan: [Claude#taa-paket-T1 2026-11-05]
"""paket_uret.py testleri: geçici klasörde sahte crate."""
import hashlib
import io
import json
import shutil
import sys
import tempfile
import unittest
from unittest import mock
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import paket_uret as pu  # noqa: E402


class Testler(unittest.TestCase):
    def setUp(self):
        self._t = tempfile.TemporaryDirectory()
        self.kok = Path(self._t.name)
        (self.kok / "Cargo.toml").write_text('[workspace]\nmembers=["crates/*"]\n[workspace.package]\nversion="9.9.9"\n')
        self.crate = self.kok / "crates" / "taa-x"
        self.crate.mkdir(parents=True)
        self.yaz()
        self.exe = self.kok / "taax.exe"
        self.exe.write_bytes(b"MZ-sahte")
        self.cikti = self.kok / "cikti"

    def tearDown(self):
        self._t.cleanup()

    def yaz(self, surum="0.1.0", cargo=None, modul=None):
        (self.crate / "Cargo.toml").write_text(cargo or f'[package]\nname="taa-x"\nversion="{surum}"\n')
        (self.crate / "modul.toml").write_text(modul or (
            'ad="taa-x"\nsurum="0.1.0"\narayuz="1"\nprogram="taax"\n'
            'gerektirir={ "taa-ayar" = "1" }\nsunar=["x.yap@1"]\nveri_sahibi=["x"]\n'))

    def test_uretir(self):
        p = pu.uret(self.crate, self.cikti, self.exe)
        j = json.loads((self.cikti / "paket.json").read_text(encoding="utf-8"))
        self.assertEqual(j, p)
        self.assertEqual(j["ad"], "taa-x")
        self.assertEqual(j["gerektirir"], {})  # ikili: crate bağımlılıkları statik
        self.assertEqual(j["ister"], [])

    def test_bin_yoksa_gerektirir_modulden(self):
        j = pu.uret(self.crate, self.cikti, None)
        self.assertEqual(j["gerektirir"], {"taa-ayar": "1"})

    def test_bin_ile_calisma_gerektirir(self):
        self.yaz(modul='ad="taa-x"\nsurum="0.1.0"\narayuz="1"\nprogram="taax"\n'
                       'gerektirir={ "taa-ayar" = "1" }\ncalisma_gerektirir={ "taa-y" = "2" }\n')
        j = pu.uret(self.crate, self.cikti, self.exe)
        self.assertEqual(j["gerektirir"], {"taa-y": "2"})
        self.yaz(modul='ad="taa-x"\nsurum="0.1.0"\narayuz="1"\nprogram="taax"\ncalisma_gerektirir="x"\n')
        with self.assertRaises(pu.PaketHata):
            pu.uret(self.crate, self.kok / "c2", self.exe)
        self.assertEqual(j["dosyalar"], [{"yol": "bin/taax.exe", "sha256": hashlib.sha256(b"MZ-sahte").hexdigest()}])
        self.assertEqual((self.cikti / "bin" / "taax.exe").read_bytes(), b"MZ-sahte")
        self.assertIn("icerik_sha256_tanim", j)

    def test_icerik_tanimi(self):
        d = [{"yol": "b", "sha256": "2" * 64}, {"yol": "a/c", "sha256": "1" * 64}]
        beklenen = hashlib.sha256(f"a/c\n{'1'*64}\nb\n{'2'*64}\n".encode()).hexdigest()
        self.assertEqual(pu.icerik_sha256(d), beklenen)
        self.assertEqual(pu.icerik_sha256([]), hashlib.sha256(b"").hexdigest())
        p = pu.uret(self.crate, self.cikti, self.exe)
        self.assertEqual(p["icerik_sha256"], pu.icerik_sha256(p["dosyalar"]))

    def test_bin_yoksa_bos_dosya(self):
        p = pu.uret(self.crate, self.cikti)
        self.assertEqual(p["dosyalar"], [])

    def test_calisma_alani_surumu(self):
        self.yaz(cargo='[package]\nname="taa-x"\nversion.workspace=true\n',
                 modul='ad="taa-x"\nsurum="9.9.9"\narayuz="1"\nprogram="ortak"\n')
        self.assertEqual(pu.uret(self.crate, self.cikti)["surum"], "9.9.9")

    def test_surum_uyusmaz(self):
        self.yaz(surum="0.2.0")
        with self.assertRaises(pu.PaketHata):
            pu.uret(self.crate, self.cikti)
        self.assertFalse(self.cikti.exists())

    def test_zorunlu_alan_eksik(self):
        self.yaz(modul='ad="taa-x"\nsurum="0.1.0"\n')
        with self.assertRaises(pu.PaketHata):
            pu.uret(self.crate, self.cikti)

    def test_dolu_cikti_ezilmez(self):
        self.cikti.mkdir()
        (self.cikti / "eski.txt").write_text("var")
        with self.assertRaises(pu.PaketHata):
            pu.uret(self.crate, self.cikti)
        self.assertEqual((self.cikti / "eski.txt").read_text(), "var")

    def test_yol_gecerli(self):
        for y in ("bin/a.exe", "a"):
            self.assertTrue(pu.yol_gecerli(y), y)
        for y in ("", "../a", "a/../b", "/a", "C:/a", "a\\b", "a\0b", "a//b", "./a", "a\nb", "a\tb", "a\x1fb", "a\x7fb"):
            self.assertFalse(pu.yol_gecerli(y), repr(y))

    def test_ad_surum_kurallari(self):
        for a in ("taa-x", "a", "a.b", "a+b", "comx", "a" * 64):
            self.assertTrue(pu.ad_gecerli(a), a)
        for a in ("", "A", "a_b", ".a", "a.", "-a", "a-", "a..b", "+a", "con", "prn", "aux", "nul",
                  "com1", "com9", "lpt1", "lpt9", "con.txt", "a b", "a/b", "a\0", "a" * 65, "ü"):
            self.assertFalse(pu.ad_gecerli(a), repr(a))
        for s in ("0.1.0", "1", "1.0.0-rc1", "2.0+yapi5"):
            self.assertTrue(pu.surum_gecerli(s), s)
        for s in ("", "1.", "1-", ".1", "1..2", "1.0.0-RC1", "1_0", "v1", "1/2", "1\x01", "1" + "0" * 64):
            self.assertFalse(pu.surum_gecerli(s), repr(s))

    def test_gecersiz_ad_surum_cikis_2(self):
        for modul in (
            'ad="Taa-X"\nsurum="0.1.0"\narayuz="1"\nprogram="taax"\n',
            'ad="con"\nsurum="0.1.0"\narayuz="1"\nprogram="taax"\n',
            'ad="taa-x"\nsurum="0.1.0"\narayuz="1"\nprogram="taax"\ngerektirir={ "A_b" = "1" }\n',
        ):
            self.yaz(modul=modul)
            with self.assertRaises(pu.PaketHata):
                pu.uret(self.crate, self.cikti)
            self.assertFalse(self.cikti.exists())
            with redirect_stderr(io.StringIO()):
                self.assertEqual(pu.main([str(self.crate), str(self.cikti)]), 2)
        self.yaz(surum="0.1.0..1", modul='ad="taa-x"\nsurum="0.1.0..1"\narayuz="1"\nprogram="taax"\n')
        with self.assertRaises(pu.PaketHata):
            pu.uret(self.crate, self.cikti)

    def test_sha_kopyalanan_dosyadan(self):
        gercek = shutil.copyfile

        def bozarak_kopyala(kaynak, hedef, *a, **k):
            gercek(kaynak, hedef, *a, **k)
            Path(hedef).write_bytes(b"KOPYADA-DEGISTI")
            return hedef

        with mock.patch.object(pu.shutil, "copyfile", bozarak_kopyala):
            p = pu.uret(self.crate, self.cikti, self.exe)
        self.assertEqual(p["dosyalar"][0]["sha256"], hashlib.sha256(b"KOPYADA-DEGISTI").hexdigest())
        self.assertEqual(p["dosyalar"][0]["sha256"], pu.sha256_dosya(self.cikti / "bin" / "taax.exe"))

    def test_bin_dosya_degil(self):
        with self.assertRaises(pu.PaketHata):
            pu.uret(self.crate, self.cikti, self.kok)

    def test_cli_cikis_kodlari(self):
        with redirect_stdout(io.StringIO()):
            self.assertEqual(pu.main([str(self.crate), str(self.cikti), "--bin", str(self.exe)]), 0)
        with redirect_stderr(io.StringIO()):
            self.assertEqual(pu.main([str(self.crate), str(self.cikti)]), 2)  # dolu

    def test_zip_uretir(self):
        z = self.kok / "cikti.zip"
        p = pu.uret_zip(self.crate, z, self.exe)
        import zipfile
        with zipfile.ZipFile(z) as a:
            self.assertEqual(sorted(a.namelist()), ["bin/taax.exe", "paket.json"])  # paket.json kökte
            self.assertTrue(all(i.compress_type == zipfile.ZIP_DEFLATED for i in a.infolist()))
            self.assertIsNone(a.testzip())
            j = json.loads(a.read("paket.json").decode("utf-8"))
            self.assertEqual(a.read("bin/taax.exe"), b"MZ-sahte")
        self.assertEqual(j, p)
        self.assertNotIn("imza", j)  # imzayı taa-imza atar
        self.assertEqual(j["dosyalar"], [{"yol": "bin/taax.exe", "sha256": hashlib.sha256(b"MZ-sahte").hexdigest()}])
        # klasör paketiyle aynı içerik
        k = pu.uret(self.crate, self.cikti, self.exe)
        self.assertEqual(k, p)

    def test_zip_ezmez_ve_cli(self):
        z = self.kok / "cikti.zip"
        z.write_bytes(b"var")
        with self.assertRaises(pu.PaketHata):
            pu.uret_zip(self.crate, z, self.exe)
        self.assertEqual(z.read_bytes(), b"var")
        z2 = self.kok / "yeni.zip"
        with redirect_stdout(io.StringIO()):
            self.assertEqual(pu.main([str(self.crate), "--zip", str(z2), "--bin", str(self.exe)]), 0)
        self.assertTrue(z2.is_file())
        with redirect_stderr(io.StringIO()):
            self.assertEqual(pu.main([str(self.crate), "--zip", str(z2)]), 2)  # var
            with self.assertRaises(SystemExit):  # klasör ve --zip birlikte
                pu.main([str(self.crate), str(self.cikti), "--zip", str(self.kok / "x.zip")])

    def test_imza_dosyasi_yazilir_ve_bicim_denetlenir(self):
        im = {"alg": "ed25519", "anahtar_kimlik": "ab" * 8, "deger": "cd" * 64}
        f = self.kok / "imza.json"
        f.write_text(json.dumps(im))
        z = self.kok / "imzali.zip"
        with redirect_stdout(io.StringIO()):
            self.assertEqual(pu.main([str(self.crate), "--zip", str(z), "--bin", str(self.exe), "--imza-dosya", str(f)]), 0)
        import zipfile
        with zipfile.ZipFile(z) as a:
            self.assertEqual(json.loads(a.read("paket.json"))["imza"], im)
        for kotu in ({"alg": "rsa", "anahtar_kimlik": "ab" * 8, "deger": "cd" * 64},
                     {"alg": "ed25519", "anahtar_kimlik": "ab" * 8, "deger": "zz"},
                     {"alg": "ed25519", "deger": "cd" * 64}):
            f.write_text(json.dumps(kotu))
            with self.assertRaises(pu.PaketHata):
                pu.imza_oku(f)


if __name__ == "__main__":
    unittest.main()
