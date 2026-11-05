"""Fiş logosunu 80 mm termal yazıcı için 576 nokta genişlikte 1 bit bitmap'e çevirir (logo-576.bin).

Kullanım: python3 logo-uret.py [logo.png] [içerik_genişliği_nokta]
Çıktı biçimi: 2 bayt yükseklik (küçük-endian) + satır başına 72 bayt, MSB solda, 1 = siyah.
Yazıcıya GS v 0 ile gönderilir; ortalama beyaz dolguyla bitmap içinde yapılır.
"""
import sys
from PIL import Image

kaynak = sys.argv[1] if len(sys.argv) > 1 else "terziademaltun-yazi-logosu.png"
icerik = int(sys.argv[2]) if len(sys.argv) > 2 else 440
GENISLIK = 576

im = Image.open(kaynak).convert("RGBA")
zemin = Image.new("RGBA", im.size, (255, 255, 255, 255))
im = Image.alpha_composite(zemin, im).convert("L")
oran = icerik / im.width
im = im.resize((icerik, max(1, round(im.height * oran))), Image.LANCZOS)
im = im.point(lambda p: 0 if p < 140 else 255)  # sabit eşik: düz siyah-beyaz logo için en temizi
kanvas = Image.new("L", (GENISLIK, im.height), 255)
kanvas.paste(im, ((GENISLIK - icerik) // 2, 0))
veri = bytearray([kanvas.height & 255, kanvas.height >> 8])
px = kanvas.load()
for y in range(kanvas.height):
    for xb in range(GENISLIK // 8):
        b = 0
        for k in range(8):
            if px[xb * 8 + k, y] == 0:
                b |= 0x80 >> k
        veri.append(b)
open("logo-576.bin", "wb").write(veri)
print(kanvas.size, len(veri), "bayt")
