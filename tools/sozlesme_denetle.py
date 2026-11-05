#!/usr/bin/env python3
# TAA modül sözleşmesi denetimi (PLAN-TAA-PAKET §18.3, T0). Çalıştır: python3 -I tools/sozlesme_denetle.py [--siki] [--json]
# Hata (çıkış 1): ad/Cargo uyuşmazlığı, sürüm uyuşmazlığı, bilinmeyen gereklilik, döngü, biçim hataları, ortak modülün program bağımlılığı.
# Uyarı: bir programın başka PROGRAMA ait crate'e bağımlı olması (ince çizgi kuralı). --siki ile uyarı da hata sayılır.
import json, re, sys, tomllib
from pathlib import Path

KOK = Path(__file__).resolve().parent.parent
AD_RE = re.compile(r"^[a-z][a-z0-9]*(-[a-z0-9]+)*$")
SURUM_RE = re.compile(r"^\d+\.\d+\.\d+([-+][0-9A-Za-z.-]+)?$")
YETENEK_RE = re.compile(r"^[a-z0-9-]+(\.[a-z0-9-]+)?(@\d+)?$")


def yukle():
    moduller, hatalar = {}, []
    for yol in sorted((KOK / "crates").glob("*/modul.toml")):
        try:
            m = tomllib.loads(yol.read_text(encoding="utf-8"))
        except Exception as e:  # noqa: BLE001
            hatalar.append(f"{yol.parent.name}: modul.toml okunamadı ({e})")
            continue
        m["_klasor"] = yol.parent
        moduller[m.get("ad", yol.parent.name)] = m
    for k in sorted((KOK / "crates").iterdir()):
        if k.is_dir() and not (k / "modul.toml").exists():
            hatalar.append(f"{k.name}: modul.toml yok")
    return moduller, hatalar


def cargo_surum(klasor):
    try:
        c = tomllib.loads((klasor / "Cargo.toml").read_text(encoding="utf-8"))
        return c.get("package", {}).get("version")
    except Exception:  # noqa: BLE001
        return None


def denetle():
    moduller, hatalar = yukle()
    uyarilar = []
    for ad, m in moduller.items():
        k = m["_klasor"].name
        if ad != k:
            hatalar.append(f"{k}: ad '{ad}' klasör adıyla aynı değil")
        if not AD_RE.match(ad):
            hatalar.append(f"{ad}: ad biçimi geçersiz")
        s = m.get("surum", "")
        if not SURUM_RE.match(str(s)):
            hatalar.append(f"{ad}: surum geçersiz '{s}'")
        cs = cargo_surum(m["_klasor"])
        if cs is not None and cs != s:
            hatalar.append(f"{ad}: surum {s} ≠ Cargo.toml {cs}")
        if not str(m.get("arayuz", "")).isdigit():
            hatalar.append(f"{ad}: arayuz ana sürüm sayısı olmalı")
        if not isinstance(m.get("gerektirir"), dict):
            hatalar.append(f"{ad}: gerektirir tablo olmalı")
        oz = m.get("ozellikler", [])
        if not isinstance(oz, list) or len(set(oz)) != len(oz):
            hatalar.append(f"{ad}: ozellikler liste olmalı ve yinelenmemeli")
        for y in oz:
            if not YETENEK_RE.match(str(y)):
                hatalar.append(f"{ad}: özellik adı geçersiz '{y}'")
        if not m.get("program"):
            hatalar.append(f"{ad}: program alanı yok ('ortak' ya da program adı)")
    for ad, m in moduller.items():
        g = m.get("gerektirir", {}) if isinstance(m.get("gerektirir"), dict) else {}
        for hedef, alt in g.items():
            h = moduller.get(hedef)
            if h is None:
                hatalar.append(f"{ad}: gerektirir '{hedef}' paketinde yok")
                continue
            if not str(alt).isdigit():
                hatalar.append(f"{ad}: '{hedef}' en düşük arayüz sürümü sayı olmalı")
            elif int(alt) > int(h.get("arayuz", 0)):
                hatalar.append(f"{ad}: '{hedef}' arayuz {alt} ister, kurulu {h.get('arayuz')}")
            pm, ph = m.get("program"), h.get("program")
            if pm == "ortak" and ph != "ortak":
                hatalar.append(f"{ad}: ortak modül '{hedef}' ({ph} programına ait) crate'ine bağımlı olamaz")
            elif pm != "ortak" and ph not in ("ortak", pm):
                uyarilar.append(f"{ad} ({pm}) → {hedef} ({ph}): programdan programa bağ; ortak çekirdeğe taşı ya da isteğe bağlı yetenek yap")
    # döngü
    durum = {}
    def gez(a, yol):
        if durum.get(a) == 2:
            return
        if durum.get(a) == 1:
            hatalar.append("döngü: " + " → ".join(yol[yol.index(a):] + [a]))
            return
        durum[a] = 1
        for h in (moduller.get(a, {}).get("gerektirir") or {}):
            if h in moduller:
                gez(h, yol + [h])
        durum[a] = 2
    for a in moduller:
        gez(a, [a])
    # program.toml sahiplik (varsa): her veri alanının tek sahibi
    sahipler = {}
    for yol in sorted(KOK.glob("programlar/*/program.toml")):
        try:
            p = tomllib.loads(yol.read_text(encoding="utf-8"))
        except Exception as e:  # noqa: BLE001
            hatalar.append(f"{yol}: okunamadı ({e})")
            continue
        for alan in p.get("veri_sahibi", []):
            sahipler.setdefault(alan, []).append(p.get("ad", yol.parent.name))
        for g in (p.get("gerektirir") or {}):
            if g not in moduller:
                hatalar.append(f"{yol.parent.name}: gerektirir '{g}' crate'i yok")
    for alan, s in sahipler.items():
        if len(s) > 1:
            hatalar.append(f"veri alanı '{alan}' birden çok sahipte: {', '.join(s)}")
    for yol in sorted(KOK.glob("programlar/*/program.toml")):
        p = tomllib.loads(yol.read_text(encoding="utf-8"))
        for alan in p.get("veri_okur", []):
            if alan not in sahipler:
                hatalar.append(f"{p.get('ad')}: okuduğu veri alanı '{alan}' için sahip yok")
    return len(moduller), hatalar, uyarilar


def main():
    siki, js = "--siki" in sys.argv, "--json" in sys.argv
    n, hatalar, uyarilar = denetle()
    if js:
        print(json.dumps({"modul": n, "hata": hatalar, "uyari": uyarilar}, ensure_ascii=False, indent=1))
    else:
        for h in hatalar:
            print("HATA  ", h)
        for u in uyarilar:
            print("UYARI ", u)
        print(f"{n} modül denetlendi: {len(hatalar)} hata, {len(uyarilar)} uyarı")
    sys.exit(1 if hatalar or (siki and uyarilar) else 0)


if __name__ == "__main__":
    main()
