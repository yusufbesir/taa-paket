# Sahip: Yusuf. yazan: [Claude#taa-paket-T0 2026-11-05]
"""sozlesme_denetle.py testleri: geçici klasörde sahte crate'ler."""
import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import sozlesme_denetle as sd  # noqa: E402


class Calisma(unittest.TestCase):
    def setUp(self):
        self._t = tempfile.TemporaryDirectory()
        self.kok = Path(self._t.name)
        (self.kok / "crates").mkdir()
        (self.kok / "Cargo.toml").write_text('[workspace]\nmembers=["crates/*"]\n[workspace.package]\nversion="9.9.9"\n')

    def tearDown(self):
        self._t.cleanup()

    def crate(self, ad, modul=None, surum="0.1.0", cargo_ad=None, cargo=None, **ek):
        d = self.kok / "crates" / ad
        d.mkdir(exist_ok=True)
        d.joinpath("Cargo.toml").write_text(
            cargo if cargo is not None else f'[package]\nname = "{cargo_ad or ad}"\nversion = "{surum}"\n')
        if modul is None:
            m = {"ad": ad, "surum": surum, "arayuz": "1", "gerektirir": "{}", "program": "ortak"}
            m.update(ek)
            modul = "\n".join(f"{k} = {v if k in ('gerektirir','sunar','ister','veri_sahibi','veri_okur') else json.dumps(v)}"
                              for k, v in m.items())
        d.joinpath("modul.toml").write_text(modul)

    def sonuc(self):
        return sd.denetle(self.kok)

    def hata_var(self, parca):
        return any(parca in h for h in self.sonuc()["hatalar"])


class Testler(Calisma):
    def test_temiz(self):
        self.crate("taa-a")
        self.crate("taa-b", gerektirir='{ "taa-a" = "1" }',
                   cargo='[package]\nname = "taa-b"\nversion = "0.1.0"\n[dependencies]\ntaa-a = { path = "../taa-a" }\n')
        s = self.sonuc()
        self.assertTrue(s["basarili"], s["hatalar"])
        self.assertEqual(s["crate_sayisi"], 2)
        self.assertTrue(s["uyarilar"])  # isteğe bağlı alanlar yok -> uyarı

    def test_modul_yok(self):
        self.crate("taa-a")
        (self.kok / "crates" / "taa-a" / "modul.toml").unlink()
        self.assertTrue(self.hata_var("modul.toml dosya yok"))

    def test_zorunlu_alan_eksik_ve_bos(self):
        self.crate("taa-a", modul='ad = "taa-a"\nsurum = "0.1.0"\narayuz = ""\ngerektirir = {}\n')
        self.assertTrue(self.hata_var("eksik: program"))
        self.assertTrue(self.hata_var("boş ya da yanlış türde: arayuz"))

    def test_gerektirir_program_adi(self):
        self.crate("taa-a", gerektirir='{ "taarkod" = "1" }')
        self.assertTrue(self.hata_var("taa-' ile başlamalı"))

    def test_gerektirir_olmayan_crate(self):
        self.crate("taa-a", gerektirir='{ "taa-yok" = "1" }')
        self.assertTrue(self.hata_var("olmayan crate"))

    def test_veri_okur_sahipsiz(self):
        self.crate("taa-a", veri_okur='["urunler"]')
        self.assertTrue(self.hata_var("sahibi yok"))

    def test_veri_okur_tek_sahip(self):
        self.crate("taa-a", veri_okur='["urunler"]')
        self.crate("taa-b", veri_sahibi='["urunler"]')
        self.assertTrue(self.sonuc()["basarili"], self.sonuc()["hatalar"])

    def test_iki_sahip(self):
        self.crate("taa-a", veri_okur='["urunler"]', veri_sahibi='["urunler"]')
        self.crate("taa-b", veri_sahibi='["urunler"]')
        self.assertTrue(self.hata_var("birden çok sahip"))

    def test_yetenek_bicimi(self):
        self.crate("taa-a", sunar='["Kod.uret@1", "kod.uret", "iyi.ad@2"]', ister='["x@1"]')
        h = self.sonuc()["hatalar"]
        self.assertEqual(len([x for x in h if "biçiminde değil" in x]), 3)

    def test_sunar_benzersiz(self):
        self.crate("taa-a", sunar='["kod.uret@1"]')
        self.crate("taa-b", sunar='["kod.uret@1"]')
        self.assertTrue(self.hata_var("birden çok crate"))

    def test_surum_uyusmaz(self):
        self.crate("taa-a", modul='ad="taa-a"\nsurum="0.2.0"\narayuz="1"\ngerektirir={}\nprogram="ortak"\n',
                   cargo='[package]\nname="taa-a"\nversion="0.1.0"\n')
        self.assertTrue(self.hata_var("Cargo.toml version"))

    def test_surum_calisma_alani(self):
        self.crate("taa-a", surum="9.9.9", cargo='[package]\nname="taa-a"\nversion.workspace = true\n')
        self.assertTrue(self.sonuc()["basarili"], self.sonuc()["hatalar"])

    def test_ad_uyusmaz(self):
        self.crate("taa-a", modul='ad="taa-z"\nsurum="0.1.0"\narayuz="1"\ngerektirir={}\nprogram="ortak"\n',
                   cargo_ad="taa-y")
        self.assertTrue(self.hata_var("klasör adıyla aynı değil"))
        self.assertEqual(sum("klasör adıyla" in h for h in self.sonuc()["hatalar"]), 2)

    def test_gerektirir_cargo_birebir(self):
        self.crate("taa-a")
        self.crate("taa-b", cargo='[package]\nname = "taa-b"\nversion = "0.1.0"\n[dependencies]\ntaa-a = { path = "../taa-a" }\n')
        self.assertTrue(self.hata_var("Cargo.toml bağımlılıklarıyla birebir değil"))
        self.crate("taa-b", gerektirir='{ "taa-a" = "1" }',
                   cargo='[package]\nname = "taa-b"\nversion = "0.1.0"\n[dependencies]\ntaa-a = { path = "../taa-a" }\n')
        self.assertFalse(self.hata_var("birebir değil"))

    def test_ince_cizgi_program_bagimliligi(self):
        self.crate("taa-a", program="p1")
        self.crate("taa-b", program="p2", gerektirir='{ "taa-a" = "1" }',
                   cargo='[package]\nname = "taa-b"\nversion = "0.1.0"\n[dependencies]\ntaa-a = { path = "../taa-a" }\n')
        self.assertTrue(self.hata_var("başka programın crate'ine zorunlu bağımlı"))
        self.crate("taa-a", program="ortak")
        self.assertFalse(self.hata_var("başka programın"))

    def test_cikis_kodu_ve_json(self):
        self.crate("taa-a")
        tampon = io.StringIO()
        with redirect_stdout(tampon):
            kod = sd.main(["--kok", str(self.kok), "--json"])
        self.assertEqual(kod, 0)
        self.assertTrue(json.loads(tampon.getvalue())["basarili"])
        self.crate("taa-b", gerektirir='{ "x" = "1" }')
        with redirect_stdout(io.StringIO()):
            self.assertEqual(sd.main(["--kok", str(self.kok)]), 1)


if __name__ == "__main__":
    unittest.main()
