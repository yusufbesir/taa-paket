# Sahip: Yusuf. yazan: [Claude#taa-paket-T0 2026-11-05]
"""TAA paket sözleşme denetimi (PLAN-TAA-PAKET §18.3). Yalnız stdlib, Python 3.11+.

Kullanım: python3 tools/sozlesme_denetle.py [--kok YOL] [--json]
Hata varsa çıkış kodu 1, yoksa 0.
"""
import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

ZORUNLU = ("ad", "surum", "arayuz", "gerektirir", "program")
ISTEGE_BAGLI = ("sunar", "ister", "veri_sahibi", "veri_okur", "ayar_alani")
YETENEK_RE = re.compile(r"^[a-z][a-z0-9-]*\.[a-z][a-z0-9-]*@[0-9]+$")


def _oku(yol: Path):
    try:
        with open(yol, "rb") as f:
            return tomllib.load(f), None
    except FileNotFoundError:
        return None, "dosya yok"
    except (tomllib.TOMLDecodeError, OSError, UnicodeDecodeError, RecursionError) as e:
        return None, f"okunamadı: {type(e).__name__}: {e}"


def _bos(v) -> bool:
    return v is None or (isinstance(v, str) and not v.strip())


def _liste(v):
    return v if isinstance(v, list) else None


def _cargo_surum(cargo: dict, kok_cargo: dict):
    """(ad, surum, not) döndürür; çözülemezse surum None."""
    paket = cargo.get("package")
    if not isinstance(paket, dict):
        return None, None, "[package] bölümü yok"
    ad = paket.get("name")
    s = paket.get("version")
    if isinstance(s, dict) and s.get("workspace") is True:
        s = ((kok_cargo.get("workspace") or {}).get("package") or {}).get("version")
        if s is None:
            return ad, None, "version.workspace=true ama [workspace.package] version yok"
    return ad, s, None


def _donguler(graf: dict) -> list:
    """Yönlü grafta bulunan döngüleri (her biri bir kez, ilk düğüm tekrarlanarak) döndürür. Sıralı, deterministik."""
    bulunan, gorulen, yigin, yolda = [], set(), [], set()

    def gez(d):
        gorulen.add(d)
        yigin.append(d)
        yolda.add(d)
        for h in sorted(graf.get(d, [])):
            if h in yolda:
                dongu = yigin[yigin.index(h):]
                # normalleştir: en küçük düğümden başlat
                i = dongu.index(min(dongu))
                dongu = dongu[i:] + dongu[:i]
                dongu.append(dongu[0])
                if dongu not in bulunan:
                    bulunan.append(dongu)
            elif h not in gorulen:
                gez(h)
        yigin.pop()
        yolda.discard(d)

    for d in sorted(graf):
        if d not in gorulen:
            gez(d)
    return bulunan


def denetle(kok: Path) -> dict:
    hatalar, uyarilar = [], []
    programlar: dict = {}
    gerekler: dict = {}
    crates_dir = kok / "crates"
    klasorler = sorted(p for p in crates_dir.iterdir() if p.is_dir()) if crates_dir.is_dir() else []
    if not klasorler:
        hatalar.append(f"[paket] {crates_dir} altında crate yok")
    mevcut = {p.name for p in klasorler}
    kok_cargo, _ = _oku(kok / "Cargo.toml")
    kok_cargo = kok_cargo or {}

    modeller = {}  # klasör adı -> modul dict
    for k in klasorler:
        m, err = _oku(k / "modul.toml")
        if m is None:
            hatalar.append(f"[{k.name}] modul.toml {err}")
            continue
        modeller[k.name] = m

    sahipler: dict[str, list[str]] = {}
    sunanlar: dict[str, list[str]] = {}
    okuyanlar: list[tuple[str, str]] = []

    for kad, m in modeller.items():
        # (1) zorunlu alanlar
        for a in ZORUNLU:
            if a not in m:
                hatalar.append(f"[{kad}] zorunlu alan eksik: {a}")
            elif a != "gerektirir" and (not isinstance(m[a], str) or _bos(m[a])):
                hatalar.append(f"[{kad}] zorunlu alan boş ya da yanlış türde: {a}")
        ger = m.get("gerektirir")
        # (2) gerektirir
        if "gerektirir" in m:
            if not isinstance(ger, dict):
                hatalar.append(f"[{kad}] gerektirir tablo olmalı")
            else:
                for anahtar, aralik in ger.items():
                    if not anahtar.startswith("taa-"):
                        hatalar.append(f"[{kad}] gerektirir anahtarı 'taa-' ile başlamalı (program adı yazılamaz): {anahtar}")
                    elif anahtar not in mevcut:
                        hatalar.append(f"[{kad}] gerektirir çalışma alanında olmayan crate'i gösteriyor: {anahtar}")
                    elif anahtar == kad:
                        hatalar.append(f"[{kad}] gerektirir kendini gösteriyor")
                    if not isinstance(aralik, str) or _bos(aralik):
                        hatalar.append(f"[{kad}] gerektirir.{anahtar} sürüm aralığı boş ya da yanlış türde")
        # yeni alanlar
        eksik = [a for a in ISTEGE_BAGLI if a not in m]
        if eksik:
            uyarilar.append(f"[{kad}] isteğe bağlı alan yok (boş sayıldı): {', '.join(eksik)}")
        for a in ("sunar", "ister", "veri_sahibi", "veri_okur"):
            if a in m and _liste(m[a]) is None:
                hatalar.append(f"[{kad}] {a} dizi olmalı")
        if "ayar_alani" in m and not isinstance(m["ayar_alani"], str):
            hatalar.append(f"[{kad}] ayar_alani metin olmalı")
        # (4) yetenek biçimi
        for a in ("sunar", "ister"):
            for y in _liste(m.get(a)) or []:
                if not isinstance(y, str) or not YETENEK_RE.match(y):
                    hatalar.append(f"[{kad}] {a} girdisi 'ad.fiil@N' biçiminde değil: {y!r}")
                elif a == "sunar":
                    sunanlar.setdefault(y, []).append(kad)
        programlar[kad] = m.get("program") if isinstance(m.get("program"), str) else None
        gerekler[kad] = sorted(m["gerektirir"].keys()) if isinstance(m.get("gerektirir"), dict) else []
        for s in _liste(m.get("veri_sahibi")) or []:
            sahipler.setdefault(str(s), []).append(kad)
        for o in _liste(m.get("veri_okur")) or []:
            okuyanlar.append((kad, str(o)))
        # (5)(6) Cargo.toml
        cargo, err = _oku(crates_dir / kad / "Cargo.toml")
        if cargo is None:
            hatalar.append(f"[{kad}] Cargo.toml {err}")
            continue
        cad, csurum, not_ = _cargo_surum(cargo, kok_cargo)
        if not_:
            hatalar.append(f"[{kad}] Cargo.toml: {not_}")
        if csurum is not None and isinstance(m.get("surum"), str) and m["surum"] != csurum:
            hatalar.append(f"[{kad}] surum ({m['surum']}) Cargo.toml version ({csurum}) ile aynı değil")
        if isinstance(m.get("ad"), str) and m["ad"] != kad:
            hatalar.append(f"[{kad}] ad ({m['ad']}) klasör adıyla aynı değil")
        if cad is not None and cad != kad:
            hatalar.append(f"[{kad}] Cargo.toml name ({cad}) klasör adıyla aynı değil")
        # (7) gerektirir = Cargo [dependencies] içindeki taa-* crate'ler, birebir (dev-dependencies hariç)
        deps = cargo.get("dependencies") if isinstance(cargo.get("dependencies"), dict) else {}
        cargo_taa = sorted(d for d in deps if d.startswith("taa-"))
        if gerekler.get(kad) is not None and cargo_taa != gerekler[kad]:
            eksik = sorted(set(cargo_taa) - set(gerekler[kad]))
            fazla = sorted(set(gerekler[kad]) - set(cargo_taa))
            parca = []
            if eksik:
                parca.append("modul.toml'da eksik: " + ", ".join(eksik))
            if fazla:
                parca.append("Cargo.toml'da yok: " + ", ".join(fazla))
            hatalar.append(f"[{kad}] gerektirir Cargo.toml bağımlılıklarıyla birebir değil ({'; '.join(parca)})")

    # (8) ince çizgi: gerektirir hedefi 'ortak' ya da aynı programın crate'i olmalı (program→program zorunlu bağımlılık yasak)
    for kad, hedefler in sorted(gerekler.items()):
        for h in hedefler:
            hp = programlar.get(h)
            if h in programlar and hp not in (None, "ortak") and hp != programlar.get(kad):
                hatalar.append(f"[{kad}] (program={programlar.get(kad)}) başka programın crate'ine zorunlu bağımlı: {h} (program={hp}); yetenek (ister) ya da ortak crate olmalı")
    # (9) gerektirir grafında döngü
    for dongu in _donguler({k: [h for h in v if h in modeller and h != k] for k, v in gerekler.items()}):
        hatalar.append("[paket] gerektirir grafında döngü: " + " -> ".join(dongu))
    # (10) istenen arayüz ana sürümü = hedef modülün arayuz değeri
    for kad, m in sorted(modeller.items()):
        ger = m.get("gerektirir")
        if not isinstance(ger, dict):
            continue
        for h, istenen in sorted(ger.items()):
            hedef = modeller.get(h)
            if hedef is None or h == kad or not isinstance(istenen, str) or not isinstance(hedef.get("arayuz"), str):
                continue
            if istenen.strip() != hedef["arayuz"].strip():
                hatalar.append(f"[{kad}] gerektirir.{h} arayüz ana sürümü ({istenen}) hedefin arayuz değeriyle ({hedef['arayuz']}) aynı değil")
    # (3) veri sahipliği
    for alan, liste in sorted(sahipler.items()):
        if len(liste) > 1:
            hatalar.append(f"[paket] veri alanı '{alan}' için birden çok sahip: {', '.join(liste)}")
    for kad, alan in okuyanlar:
        n = len(sahipler.get(alan, []))
        if n == 0:
            hatalar.append(f"[{kad}] veri_okur '{alan}' alanının sahibi yok")
    # (4) sunar benzersizliği
    for y, liste in sorted(sunanlar.items()):
        if len(liste) > 1:
            hatalar.append(f"[paket] yetenek '{y}' birden çok crate tarafından sunuluyor: {', '.join(liste)}")

    return {
        "kok": str(kok),
        "crate_sayisi": len(klasorler),
        "temiz_crate_sayisi": len(klasorler) - len({h.split("]")[0][1:] for h in hatalar if h.startswith("[") and not h.startswith("[paket]")}),
        "hatalar": hatalar,
        "uyarilar": uyarilar,
        "basarili": not hatalar,
    }


def main(argv=None) -> int:
    varsayilan = Path(__file__).resolve().parent.parent
    ap = argparse.ArgumentParser(description="TAA paket sözleşme denetimi (§18.3)")
    ap.add_argument("--kok", type=Path, default=varsayilan, help="taa-paket kökü (varsayılan: betiğin iki üst klasörü)")
    ap.add_argument("--json", action="store_true", help="çıktıyı JSON ver")
    a = ap.parse_args(argv)
    s = denetle(a.kok.resolve())
    if a.json:
        print(json.dumps(s, ensure_ascii=False, indent=2))
    else:
        for u in s["uyarilar"]:
            print(f"UYARI {u}")
        for h in s["hatalar"]:
            print(f"HATA  {h}")
        print(f"Özet: {s['temiz_crate_sayisi']}/{s['crate_sayisi']} crate temiz, "
              f"{len(s['hatalar'])} hata, {len(s['uyarilar'])} uyarı. "
              f"Sonuç: {'GEÇTİ' if s['basarili'] else 'KALDI'}")
    return 0 if s["basarili"] else 1


if __name__ == "__main__":
    sys.exit(main())
