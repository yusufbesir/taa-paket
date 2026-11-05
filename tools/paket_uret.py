# Sahip: Yusuf. yazan: [Claude#taa-paket-T1 2026-11-05]
"""modul.toml + Cargo sürümü -> klasör paketi (paket.json). T1-TASARIM §1.2 ve §3. Yalnız stdlib.

Kullanım: python3 tools/paket_uret.py <crate-klasoru> <cikti-klasoru> [--bin <exe>]
          python3 tools/paket_uret.py <crate-klasoru> --zip <cikti.zip> [--bin <exe>]

icerik_sha256 tanımı (Rust tarafı aynısını kullanır): dosyalar yol sırasıyla (UTF-8 bayt sırası),
her dosya için "yol\\nsha256\\n" metni art arda eklenir; birleşimin SHA256'sı onaltılık küçük harf.
Çıktı klasörü yoksa oluşturulur, varsa BOŞ olmalıdır (hiçbir şey ezilmez ya da silinmez).
`--zip` aynı içeriği tek zip olarak üretir (paket.json zipin KÖKÜNDE; deflate; çıktı zip var olamaz).
`imza` alanı YAZILMAZ: imzayı `taa-imza` atar (imzalanan veri `icerik_sha256` metnidir). Zip için hazır imza
`--imza-dosya <imza.json>` ({"alg":"ed25519","anahtar_kimlik":..,"deger":hex}) ile verilir; araç yalnız biçimini denetler.
"""
import argparse
import hashlib
import json
import re
import shutil
import sys
import tomllib
import zipfile
from pathlib import Path

ICERIK_TANIM = 'yol sırasıyla (UTF-8 bayt sırası) her dosya için "yol\\nsha256\\n" birleşiminin SHA256\'sı'
MAKS_DOSYA = 10_000
MAKS_TOPLAM = 2 * 1024 * 1024 * 1024


class PaketHata(Exception):
    pass


def _toml(yol: Path) -> dict:
    try:
        with open(yol, "rb") as f:
            return tomllib.load(f)
    except FileNotFoundError:
        raise PaketHata(f"{yol} yok")
    except (tomllib.TOMLDecodeError, OSError, UnicodeDecodeError) as e:
        raise PaketHata(f"{yol} okunamadı: {e}")


def _kok_cargo(crate: Path) -> dict:
    for ust in crate.resolve().parents:
        c = ust / "Cargo.toml"
        if c.is_file():
            d = _toml(c)
            if "workspace" in d:
                return d
    return {}


def cargo_surum(crate: Path) -> str:
    cargo = _toml(crate / "Cargo.toml")
    paket = cargo.get("package")
    if not isinstance(paket, dict):
        raise PaketHata("Cargo.toml: [package] bölümü yok")
    s = paket.get("version")
    if isinstance(s, dict) and s.get("workspace") is True:
        s = ((_kok_cargo(crate).get("workspace") or {}).get("package") or {}).get("version")
    if not isinstance(s, str) or not s:
        raise PaketHata("Cargo.toml: sürüm çözülemedi")
    return s


def sha256_dosya(yol: Path) -> str:
    h = hashlib.sha256()
    with open(yol, "rb") as f:
        for parca in iter(lambda: f.read(1 << 20), b""):
            h.update(parca)
    return h.hexdigest()


def icerik_sha256(dosyalar: list) -> str:
    h = hashlib.sha256()
    for d in sorted(dosyalar, key=lambda d: d["yol"].encode("utf-8")):
        h.update(f"{d['yol']}\n{d['sha256']}\n".encode("utf-8"))
    return h.hexdigest()


WINDOWS_AYRILMIS = frozenset(
    ["con", "prn", "aux", "nul"] + [f"com{i}" for i in range(1, 10)] + [f"lpt{i}" for i in range(1, 10)]
)
_PARCA_RE = re.compile(r"[a-z0-9.+-]{1,64}")


def _parca_gecerli(s: str) -> bool:
    """Rust paket.rs parca_gecerli ile aynı kural: yalnız [a-z0-9.+-], <=64, baş/son '.'/'-' yok, '..' yok, Windows adı yok."""
    if not isinstance(s, str) or _PARCA_RE.fullmatch(s) is None:
        return False
    if s[0] in ".-" or s[-1] in ".-" or ".." in s:
        return False
    return s.split(".")[0] not in WINDOWS_AYRILMIS


def ad_gecerli(ad: str) -> bool:
    return _parca_gecerli(ad) and ad[0] != "+"


def surum_gecerli(s: str) -> bool:
    return _parca_gecerli(s) and s[0] in "0123456789"


def yol_gecerli(yol: str) -> bool:
    if not yol or "\\" in yol or yol.startswith("/") or ":" in yol:
        return False
    if any(ord(c) < 0x20 or ord(c) == 0x7F for c in yol):  # NUL dahil tüm denetim karakterleri
        return False
    return all(p not in ("", ".", "..") for p in yol.split("/"))


def _hazirla(crate: Path, bin_yol: Path | None) -> tuple:
    """modul.toml'u doğrular; (modül sözlüğü, gerektirir, [(kaynak, hedef göreli yol)]) döner. Hiçbir şey yazmaz."""
    crate = Path(crate)
    m = _toml(crate / "modul.toml")
    for a in ("ad", "surum", "arayuz", "program"):
        if not isinstance(m.get(a), str) or not m[a].strip():
            raise PaketHata(f"modul.toml: zorunlu alan eksik ya da boş: {a}")
    if not ad_gecerli(m["ad"]):
        raise PaketHata(f"modul.toml: geçersiz ad: {m['ad']!r}")
    if not surum_gecerli(m["surum"]):
        raise PaketHata(f"modul.toml: geçersiz surum: {m['surum']!r}")
    ger = m.get("gerektirir", {})
    if not isinstance(ger, dict):
        raise PaketHata("modul.toml: gerektirir tablo olmalı")
    for g in ger:
        if not ad_gecerli(g):
            raise PaketHata(f"modul.toml: geçersiz gerektirir adı: {g!r}")
    surum = cargo_surum(crate)
    if surum != m["surum"]:
        raise PaketHata(f"modul.toml surum ({m['surum']}) Cargo.toml version ({surum}) ile aynı değil")
    for a in ("sunar", "ister", "veri_sahibi", "veri_okur"):
        if not isinstance(m.get(a, []), list):
            raise PaketHata(f"modul.toml: {a} dizi olmalı")

    kopyalar = []  # (kaynak, hedef göreli yol)
    if bin_yol is not None:
        bin_yol = Path(bin_yol)
        if not bin_yol.is_file() or bin_yol.is_symlink():
            raise PaketHata(f"--bin düz dosya olmalı: {bin_yol}")
        hedef = f"bin/{bin_yol.name}"
        if not yol_gecerli(hedef):
            raise PaketHata(f"geçersiz dosya adı: {bin_yol.name}")
        kopyalar.append((bin_yol, hedef))
    if len(kopyalar) > MAKS_DOSYA:
        raise PaketHata("dosya sayısı sınırı aşıldı")
    if sum(k.stat().st_size for k, _ in kopyalar) > MAKS_TOPLAM:
        raise PaketHata("toplam boyut sınırı aşıldı")
    return m, ger, kopyalar


def imza_oku(yol: Path) -> dict:
    """Hazır imza dosyasını okur; yalnız biçimi denetler (doğrulamayı Rust taa-kur yapar)."""
    try:
        j = json.loads(Path(yol).read_text(encoding="utf-8"))
    except (OSError, ValueError) as e:
        raise PaketHata(f"imza dosyası okunamadı: {e}")
    if not isinstance(j, dict) or set(j) != {"alg", "anahtar_kimlik", "deger"}:
        raise PaketHata("imza dosyası {alg, anahtar_kimlik, deger} alanlarını içermeli")
    if j["alg"] != "ed25519":
        raise PaketHata("imza algoritması ed25519 olmalı")
    if not (isinstance(j["deger"], str) and re.fullmatch(r"[0-9a-f]{128}", j["deger"])):
        raise PaketHata("imza değeri 128 haneli küçük harf onaltılık olmalı")
    if not (isinstance(j["anahtar_kimlik"], str) and re.fullmatch(r"[0-9a-f]{16}", j["anahtar_kimlik"])):
        raise PaketHata("anahtar_kimlik 16 haneli küçük harf onaltılık olmalı")
    return j


def _paket_sozlugu(m: dict, ger: dict, dosyalar: list, imza: dict | None = None) -> dict:
    p = {
        "sema": 1,
        "ad": m["ad"],
        "surum": m["surum"],
        "arayuz": m["arayuz"],
        "program": m["program"],
        "gerektirir": dict(sorted(ger.items())),
        "sunar": list(m.get("sunar", [])),
        "ister": list(m.get("ister", [])),
        "veri_sahibi": list(m.get("veri_sahibi", [])),
        "veri_okur": list(m.get("veri_okur", [])),
        "dosyalar": dosyalar,
        "icerik_sha256": icerik_sha256(dosyalar),
        "icerik_sha256_tanim": ICERIK_TANIM,
    }
    if imza is not None:
        p["imza"] = imza
    return p


def uret(crate: Path, cikti: Path, bin_yol: Path | None = None, imza: dict | None = None) -> dict:
    m, ger, kopyalar = _hazirla(crate, bin_yol)
    cikti = Path(cikti)
    if cikti.exists() and (not cikti.is_dir() or any(cikti.iterdir())):
        raise PaketHata(f"çıktı klasörü boş değil (ezilmez): {cikti}")

    dosyalar = []
    cikti.mkdir(parents=True, exist_ok=True)
    for kaynak, hedef in kopyalar:
        h = cikti / hedef
        h.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(kaynak, h)
        # SHA256 kopyalanan dosyadan hesaplanır: paket.json, gerçekte çıktıdaki baytları tanımlar.
        dosyalar.append({"yol": hedef, "sha256": sha256_dosya(h)})
    dosyalar.sort(key=lambda d: d["yol"].encode("utf-8"))

    paket = _paket_sozlugu(m, ger, dosyalar, imza)
    (cikti / "paket.json").write_text(json.dumps(paket, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return paket


def _zip_girdi(z: zipfile.ZipFile, ad: str, bayt: bytes) -> None:
    bilgi = zipfile.ZipInfo(ad, date_time=(1980, 1, 1, 0, 0, 0))  # sabit zaman: aynı girdi aynı zip
    bilgi.compress_type = zipfile.ZIP_DEFLATED
    bilgi.external_attr = 0o644 << 16
    z.writestr(bilgi, bayt)


def uret_zip(crate: Path, zip_yolu: Path, bin_yol: Path | None = None, imza: dict | None = None) -> dict:
    """`uret` ile aynı içeriği tek zip olarak yazar (paket.json kökte). Var olan zip ezilmez."""
    m, ger, kopyalar = _hazirla(crate, bin_yol)
    zip_yolu = Path(zip_yolu)
    if zip_yolu.exists():
        raise PaketHata(f"çıktı zip zaten var (ezilmez): {zip_yolu}")
    # SHA256 ve zip içeriği aynı baytlardan: önce oku, sonra hem özetle hem yaz.
    icerikler = []
    dosyalar = []
    for kaynak, hedef in kopyalar:
        bayt = Path(kaynak).read_bytes()
        icerikler.append((hedef, bayt))
        dosyalar.append({"yol": hedef, "sha256": hashlib.sha256(bayt).hexdigest()})
    dosyalar.sort(key=lambda d: d["yol"].encode("utf-8"))
    paket = _paket_sozlugu(m, ger, dosyalar, imza)
    zip_yolu.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(zip_yolu, "x") as z:
        _zip_girdi(z, "paket.json", (json.dumps(paket, ensure_ascii=False, indent=2) + "\n").encode("utf-8"))
        for hedef, bayt in sorted(icerikler, key=lambda x: x[0].encode("utf-8")):
            _zip_girdi(z, hedef, bayt)
    return paket


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="modul.toml -> klasör ya da zip paketi (T1-TASARIM §1.2, T2-TASARIM PR-A2)")
    ap.add_argument("crate_klasoru", type=Path)
    ap.add_argument("cikti_klasoru", type=Path, nargs="?", default=None)
    ap.add_argument("--bin", dest="bin_yol", type=Path, default=None, help="pakete konacak çalıştırılabilir (bin/ altına)")
    ap.add_argument("--zip", dest="zip_yolu", type=Path, default=None, help="klasör yerine tek zip üret (paket.json kökte)")
    ap.add_argument("--imza-dosya", dest="imza_dosya", type=Path, default=None, help="hazır imza JSON'u (taa-imza çıktısı); yalnız biçimi denetlenir")
    a = ap.parse_args(argv)
    if (a.cikti_klasoru is None) == (a.zip_yolu is None):
        ap.error("ya <cikti_klasoru> ya da --zip <cikti.zip> verilmeli (ikisi birden olamaz)")
    try:
        imza = imza_oku(a.imza_dosya) if a.imza_dosya is not None else None
        if a.zip_yolu is not None:
            p = uret_zip(a.crate_klasoru, a.zip_yolu, a.bin_yol, imza)
        else:
            p = uret(a.crate_klasoru, a.cikti_klasoru, a.bin_yol, imza)
    except PaketHata as e:
        print(f"HATA {e}", file=sys.stderr)
        return 2
    print(f"{p['ad']} {p['surum']}: {len(p['dosyalar'])} dosya, icerik_sha256={p['icerik_sha256']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
