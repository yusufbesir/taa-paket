# Sahip: Yusuf. yazan: [Claude#taa-paket-T1 2026-11-05]
"""modul.toml + Cargo sürümü -> klasör paketi (paket.json). T1-TASARIM §1.2 ve §3. Yalnız stdlib.

Kullanım: python3 tools/paket_uret.py <crate-klasoru> <cikti-klasoru> [--bin <exe>]

icerik_sha256 tanımı (Rust tarafı aynısını kullanır): dosyalar yol sırasıyla (UTF-8 bayt sırası),
her dosya için "yol\\nsha256\\n" metni art arda eklenir; birleşimin SHA256'sı onaltılık küçük harf.
Çıktı klasörü yoksa oluşturulur, varsa BOŞ olmalıdır (hiçbir şey ezilmez ya da silinmez).
"""
import argparse
import hashlib
import json
import re
import shutil
import sys
import tomllib
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


def uret(crate: Path, cikti: Path, bin_yol: Path | None = None) -> dict:
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

    cikti = Path(cikti)
    if cikti.exists() and (not cikti.is_dir() or any(cikti.iterdir())):
        raise PaketHata(f"çıktı klasörü boş değil (ezilmez): {cikti}")

    dosyalar = []
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

    cikti.mkdir(parents=True, exist_ok=True)
    for kaynak, hedef in kopyalar:
        h = cikti / hedef
        h.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(kaynak, h)
        # SHA256 kopyalanan dosyadan hesaplanır: paket.json, gerçekte çıktıdaki baytları tanımlar.
        dosyalar.append({"yol": hedef, "sha256": sha256_dosya(h)})
    dosyalar.sort(key=lambda d: d["yol"].encode("utf-8"))

    paket = {
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
    (cikti / "paket.json").write_text(json.dumps(paket, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return paket


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="modul.toml -> klasör paketi (T1-TASARIM §1.2)")
    ap.add_argument("crate_klasoru", type=Path)
    ap.add_argument("cikti_klasoru", type=Path)
    ap.add_argument("--bin", dest="bin_yol", type=Path, default=None, help="pakete konacak çalıştırılabilir (bin/ altına)")
    a = ap.parse_args(argv)
    try:
        p = uret(a.crate_klasoru, a.cikti_klasoru, a.bin_yol)
    except PaketHata as e:
        print(f"HATA {e}", file=sys.stderr)
        return 2
    print(f"{p['ad']} {p['surum']}: {len(p['dosyalar'])} dosya, icerik_sha256={p['icerik_sha256']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
