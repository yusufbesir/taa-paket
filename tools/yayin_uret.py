# Sahip: Yusuf. yazan: [Claude#taa-paket-T4 2026-11-05]
"""Paket zip'lerinden yayin.json üretir (PLAN-TAA-PAKET §16-17, docs/T3-TASARIM.md §3). Yalnız stdlib.

Kullanım: python3 tools/yayin_uret.py --cikti <yayin.json> [--kanal kararli] [--taban-url <url>]
                                     [--onceki <eski-yayin.json>] [--en-az-surum <ad>=<surum> ...] <paket.zip> ...

Her zip için kökteki paket.json okunur (ad, surum, arayuz, gerektirir); dosyanın SHA256'sı ve boyutu ölçülür.
`url`: zip'in dosya adı (kaynak klasörüne/tabana göreli); `--taban-url` verilirse `<taban>/<ad>` (mutlak).
`--onceki`: eski yayin.json'daki paketler korunur; aynı (ad, surum, kanal) yenisiyle DEĞİŞTİRİLİR. `imza` alanı
kopyalanmaz: çıktı her zaman imzasızdır, imzayı `taa-imza imzala <yayin.json> --ozel <anahtar> --tur yayin` atar.
İmzasız zip (paket.json içinde `imza` yok) uyarıyla kabul edilir: güvenilir anahtar olan kurulumlar onu REDDEDER;
`--imzali-zorunlu` ile hata olur. Çıktı dosyası varsa ezilmez (`.once-N` yedeği alınır). `yamalar` boş yazılır (T3/T4 tam dosya).
"""
import argparse
import hashlib
import json
import re
import sys
import zipfile
from pathlib import Path

AD_DESEN = re.compile(r"^[a-z0-9]+(-[a-z0-9]+)*$")
SURUM_DESEN = re.compile(r"^\d+\.\d+\.\d+$")
MAKS_PAKET_JSON = 1024 * 1024


class YayinHata(Exception):
    pass


def sha256_dosya(yol: Path) -> tuple:
    h = hashlib.sha256()
    n = 0
    with open(yol, "rb") as f:
        for parca in iter(lambda: f.read(1 << 20), b""):
            h.update(parca)
            n += len(parca)
    return h.hexdigest(), n


def paket_json_oku(zip_yolu: Path) -> dict:
    """Zip kökündeki paket.json'u döner; yoksa/bozuksa YayinHata."""
    try:
        with zipfile.ZipFile(zip_yolu) as z:
            adlar = z.namelist()
            if "paket.json" not in adlar:
                raise YayinHata(f"{zip_yolu}: kökte paket.json yok (paket zip'i değil)")
            bilgi = z.getinfo("paket.json")
            if bilgi.file_size > MAKS_PAKET_JSON:
                raise YayinHata(f"{zip_yolu}: paket.json çok büyük")
            ham = z.read("paket.json")
    except zipfile.BadZipFile as e:
        raise YayinHata(f"{zip_yolu}: zip açılamadı ({e})")
    except OSError as e:
        raise YayinHata(f"{zip_yolu}: okunamadı ({e})")
    try:
        p = json.loads(ham.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as e:
        raise YayinHata(f"{zip_yolu}: paket.json bozuk ({e})")
    if not isinstance(p, dict):
        raise YayinHata(f"{zip_yolu}: paket.json nesne değil")
    ad, surum, arayuz = p.get("ad"), p.get("surum"), p.get("arayuz")
    if not isinstance(ad, str) or not AD_DESEN.match(ad):
        raise YayinHata(f"{zip_yolu}: paket.json ad geçersiz ({ad!r})")
    if not isinstance(surum, str) or not SURUM_DESEN.match(surum):
        raise YayinHata(f"{zip_yolu}: paket.json surum geçersiz ({surum!r})")
    if not isinstance(arayuz, str) or not arayuz:
        raise YayinHata(f"{zip_yolu}: paket.json arayuz boş")
    ger = p.get("gerektirir", {})
    if not isinstance(ger, dict) or not all(isinstance(k, str) and isinstance(v, str) for k, v in ger.items()):
        raise YayinHata(f"{zip_yolu}: paket.json gerektirir tablo değil")
    return p


def girdi_uret(zip_yolu: Path, kanal: str, taban_url: str | None, en_az: dict, imzali_zorunlu: bool, uyar) -> dict:
    p = paket_json_oku(zip_yolu)
    if "imza" not in p:
        if imzali_zorunlu:
            raise YayinHata(f"{zip_yolu}: paket imzasız (--imzali-zorunlu). Öneri: önce `taa-imza imzala {zip_yolu.name}`")
        uyar(f"Uyarı: {zip_yolu.name} imzasız; güvenilir anahtarı olan kurulumlar bu paketi reddeder.")
    sha, boyut = sha256_dosya(zip_yolu)
    if boyut == 0:
        raise YayinHata(f"{zip_yolu}: boş dosya")
    url = zip_yolu.name if not taban_url else f"{taban_url.rstrip('/')}/{zip_yolu.name}"
    g = {
        "ad": p["ad"],
        "surum": p["surum"],
        "kanal": kanal,
        "arayuz": p["arayuz"],
        "gerektirir": dict(sorted(p.get("gerektirir", {}).items())),
        "tam": {"url": url, "sha256": sha, "boyut": boyut},
        "yamalar": [],
    }
    if p["ad"] in en_az:
        if surum_karsilastir(en_az[p["ad"]], p["surum"]) > 0:
            raise YayinHata(f"{p['ad']}: en_az_surum ({en_az[p['ad']]}) sürümden ({p['surum']}) büyük olamaz")
        g["en_az_surum"] = en_az[p["ad"]]
    return g


def surum_karsilastir(a: str, b: str) -> int:
    ta, tb = tuple(int(x) for x in a.split(".")), tuple(int(x) for x in b.split("."))
    return (ta > tb) - (ta < tb)


def onceki_oku(yol: Path) -> list:
    try:
        y = json.loads(yol.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as e:
        raise YayinHata(f"{yol}: eski yayin.json okunamadı ({e})")
    if not isinstance(y, dict) or y.get("sema") != 1 or not isinstance(y.get("paketler"), list):
        raise YayinHata(f"{yol}: eski yayin.json şeması tanınmadı (sema 1 bekleniyor)")
    for p in y["paketler"]:
        if not isinstance(p, dict) or not all(k in p for k in ("ad", "surum", "kanal", "tam")):
            raise YayinHata(f"{yol}: eski yayin.json'da eksik alanlı paket")
    return y["paketler"]


def birlestir(eski: list, yeni: list) -> list:
    """Aynı (ad, surum, kanal) yenisiyle değiştirilir; sıra: ad, sürüm (sayısal), kanal."""
    sozluk = {(p["ad"], p["surum"], p["kanal"]): p for p in eski}
    for p in yeni:
        sozluk[(p["ad"], p["surum"], p["kanal"])] = p
    return [sozluk[k] for k in sorted(sozluk, key=lambda k: (k[0], tuple(int(x) for x in k[1].split(".")), k[2]))]


def yedekle(yol: Path) -> Path | None:
    if not yol.exists():
        return None
    n = 1
    while True:
        y = yol.with_name(f"{yol.name}.once-{n}")
        if not y.exists():
            yol.rename(y)
            return y
        n += 1


def uret(zipler: list, cikti: Path, kanal: str = "kararli", taban_url: str | None = None, onceki: Path | None = None,
         en_az: dict | None = None, imzali_zorunlu: bool = False, uyar=lambda m: print(m, file=sys.stderr)) -> dict:
    if not AD_DESEN.match(kanal):
        raise YayinHata(f"kanal geçersiz ({kanal!r}): küçük harf, rakam ve '-'")
    if taban_url is not None and not re.match(r"^https?://\S+$", taban_url):
        raise YayinHata(f"--taban-url http:// ya da https:// ile başlamalı ({taban_url!r})")
    if not zipler:
        raise YayinHata("en az bir paket zip'i verin")
    en_az = en_az or {}
    for ad, s in en_az.items():
        if not AD_DESEN.match(ad) or not SURUM_DESEN.match(s):
            raise YayinHata(f"--en-az-surum {ad}={s} biçimi: <ad>=<x.y.z>")
    yeni = [girdi_uret(Path(z), kanal, taban_url, en_az, imzali_zorunlu, uyar) for z in zipler]
    gorulen = set()
    for g in yeni:
        k = (g["ad"], g["surum"], g["kanal"])
        if k in gorulen:
            raise YayinHata(f"{k[0]} {k[1]} ({k[2]}) iki kez verildi")
        gorulen.add(k)
    eski = onceki_oku(Path(onceki)) if onceki else []
    y = {"sema": 1, "paketler": birlestir(eski, yeni)}
    cikti = Path(cikti)
    cikti.parent.mkdir(parents=True, exist_ok=True)
    yedek = yedekle(cikti)
    if yedek:
        uyar(f"Var olan {cikti.name} ezilmedi: {yedek.name} olarak taşındı.")
    gecici = cikti.with_name(cikti.name + ".yeni")
    gecici.write_text(json.dumps(y, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    gecici.replace(cikti)
    return y


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="paket zip'lerinden yayin.json (imzasız; taa-imza ile imzalayın)")
    ap.add_argument("zipler", nargs="+", type=Path)
    ap.add_argument("--cikti", required=True, type=Path)
    ap.add_argument("--kanal", default="kararli")
    ap.add_argument("--taban-url", dest="taban_url", default=None, help="verilirse url mutlak olur: <taban>/<zip adı>")
    ap.add_argument("--onceki", type=Path, default=None, help="eski yayin.json; paketleri korunur, aynı sürüm değiştirilir")
    ap.add_argument("--en-az-surum", dest="en_az", action="append", default=[], metavar="AD=SURUM",
                    help="bu sürümün altındaki kurulumlar güncellenmek zorunda (sabitli olsa da)")
    ap.add_argument("--imzali-zorunlu", dest="imzali_zorunlu", action="store_true", help="imzasız zip hata olsun")
    a = ap.parse_args(argv)
    en_az = {}
    for e in a.en_az:
        ad, _, s = e.partition("=")
        en_az[ad] = s
    try:
        y = uret(a.zipler, a.cikti, a.kanal, a.taban_url, a.onceki, en_az, a.imzali_zorunlu)
    except YayinHata as e:
        print(f"Hata: {e}", file=sys.stderr)
        return 2
    print(f"{a.cikti}: {len(y['paketler'])} paket kaydı ({len(a.zipler)} yeni). İmzasız; imzalamak için: "
          f"taa-imza imzala {a.cikti} --ozel <anahtar> --tur yayin")
    return 0


if __name__ == "__main__":
    sys.exit(main())
