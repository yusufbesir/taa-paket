# Sahip: Yusuf. yazan: [Claude#ortak-veri-havuzu 2026-11-12]
"""TAA veri havuzu denetimi (docs/havuz-sozlesmesi.md). Yalnız stdlib, Python 3.11+.

Kullanım: python3 tools/havuz_denetle.py [--katalog docs/havuz-turleri.toml]
    [--program program.toml | modul.toml ...] [--json]
Hata varsa çıkış kodu 1, yoksa 0.
"""
import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

AD_RE = re.compile(r"^[a-z][a-z0-9-]*(\.[a-z][a-z0-9-]*)+$")
PROG_RE = re.compile(r"^[a-z0-9-]{1,32}$")
TUR_ZORUNLU = ("surum", "aile", "kip", "yazanlar", "okuyanlar")


def _oku(yol):
    try:
        with open(yol, "rb") as f:
            return tomllib.load(f), None
    except FileNotFoundError:
        return None, "dosya yok"
    except (tomllib.TOMLDecodeError, OSError, UnicodeDecodeError) as e:
        return None, f"okunamadı: {type(e).__name__}: {e}"


def _str_liste(v):
    return isinstance(v, list) and all(isinstance(x, str) for x in v)


def katalog_denetle(kat):
    h = []
    progs = (kat.get("programlar") or {}).get("adlar")
    if not _str_liste(progs) or not progs:
        return ["[programlar] adlar listesi yok"]
    for p in progs:
        if not PROG_RE.match(p):
            h.append(f"program adı geçersiz: {p!r}")
    turler = kat.get("turler")
    if not isinstance(turler, dict) or not turler:
        return h + ["[turler] boş"]
    for ad, t in sorted(turler.items()):
        if not AD_RE.match(ad):
            h.append(f"tür adı geçersiz: {ad!r}")
        if not isinstance(t, dict):
            h.append(f"[{ad}] tablo değil")
            continue
        for a in TUR_ZORUNLU:
            if a not in t:
                h.append(f"[{ad}] '{a}' eksik")
        if not isinstance(t.get("surum"), int) or isinstance(t.get("surum"), bool) or t.get("surum", 0) < 1:
            h.append(f"[{ad}] surum pozitif tam sayı olmalı")
        kip = t.get("kip")
        if kip not in ("tablo", "dosya"):
            h.append(f"[{ad}] kip 'tablo' ya da 'dosya' olmalı")
        for a in ("yazanlar", "okuyanlar"):
            v = t.get(a)
            if not _str_liste(v):
                h.append(f"[{ad}] {a} metin listesi olmalı")
                continue
            for p in v:
                if p not in progs:
                    h.append(f"[{ad}] {a} içindeki '{p}' program listesinde yok")
        y = t.get("yazanlar")
        if kip == "tablo" and _str_liste(y) and len(y) != 1:
            h.append(f"[{ad}] tablo türünün tek yazarı olmalı (şimdi {len(y)})")
        if _str_liste(y) and not y:
            h.append(f"[{ad}] yazanlar boş")
        if kip == "dosya":
            m = t.get("mime")
            if m is not None and (not _str_liste(m) or not m):
                h.append(f"[{ad}] mime boş olmayan metin listesi olmalı")
    return h


def tur_adi(girdi):
    """'urun.liste@1' -> ('urun.liste', 1); sürümsüz ad sürüm None."""
    ad, _, s = girdi.partition("@")
    if not s:
        return ad, None
    return ad, int(s) if s.isdigit() else -1


def program_denetle(m, kat, kaynak):
    h = []
    ad = m.get("ad")
    turler = kat.get("turler") or {}
    progs = (kat.get("programlar") or {}).get("adlar") or []
    if (m.get("havuz_okur") or m.get("havuz_yazar") or m.get("havuz_ilgi")) and ad not in progs:
        h.append(f"{kaynak}: '{ad}' katalogdaki program listesinde yok")
    for alan in ("havuz_okur", "havuz_yazar"):
        v = m.get(alan)
        if v is None:
            continue
        if not _str_liste(v):
            h.append(f"{kaynak}: {alan} metin listesi olmalı")
            continue
        for g in v:
            t, s = tur_adi(g)
            if t not in turler:
                h.append(f"{kaynak}: {alan} '{g}' katalogda yok")
                continue
            if s == -1 or (s is not None and s > turler[t]["surum"]):
                h.append(f"{kaynak}: {alan} '{g}' sürümü katalogdakinden ({turler[t]['surum']}) yüksek ya da geçersiz")
            if alan == "havuz_okur" and ad not in turler[t].get("okuyanlar", []):
                h.append(f"{kaynak}: '{ad}' '{t}' türünü okuyanlar listesinde değil")
            if alan == "havuz_yazar" and ad not in turler[t].get("yazanlar", []):
                h.append(f"{kaynak}: '{ad}' '{t}' türünün yazanlar listesinde değil")
    ilgi = m.get("havuz_ilgi")
    if ilgi is not None:
        aileler = {t.get("aile") for t in turler.values()}
        if not _str_liste(ilgi):
            h.append(f"{kaynak}: havuz_ilgi metin listesi olmalı")
        else:
            for a in ilgi:
                if a not in aileler:
                    h.append(f"{kaynak}: havuz_ilgi '{a}' katalogda aile değil")
    return h


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("--katalog", default=str(Path(__file__).resolve().parent.parent / "docs" / "havuz-turleri.toml"))
    ap.add_argument("--program", nargs="*", default=[])
    ap.add_argument("--json", action="store_true")
    a = ap.parse_args(argv)
    kat, e = _oku(Path(a.katalog))
    if kat is None:
        hatalar = [f"katalog {e}"]
    else:
        hatalar = katalog_denetle(kat)
        if not hatalar:
            for p in a.program:
                m, e = _oku(Path(p))
                if m is None:
                    hatalar.append(f"{p}: {e}")
                else:
                    hatalar += program_denetle(m, kat, p)
    if a.json:
        print(json.dumps({"hatalar": hatalar}, ensure_ascii=False))
    else:
        for x in hatalar:
            print("HATA:", x)
        print("havuz denetimi:", "BAŞARISIZ" if hatalar else "tamam")
    return 1 if hatalar else 0


if __name__ == "__main__":
    sys.exit(main())
