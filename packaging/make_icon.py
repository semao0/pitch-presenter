"""Рисует значок для macOS (.icns) по тем же прямоугольникам, что и pitch-presenter.svg."""
import sys
from pathlib import Path

from PIL import Image, ImageDraw

RECTS = [  # x, y, w, h, rx, цвет — из pitch-presenter.svg (сетка 128)
    (8, 16, 112, 76, 10, "#1a1d23"), (16, 24, 58, 40, 4, "#5aa9ff"), (80, 24, 32, 22, 3, "#3a4250"),
    (80, 52, 32, 4, 2, "#e8e8ea"), (80, 60, 24, 4, 2, "#e8e8ea"), (16, 72, 58, 4, 2, "#e8e8ea"),
    (16, 80, 40, 4, 2, "#8b909a"), (80, 72, 32, 12, 3, "#f0b429"), (58, 92, 12, 16, 0, "#1a1d23"),
    (36, 106, 56, 8, 4, "#1a1d23"),
]


def draw(size=1024):
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    pad = size * 100 / 1024  # сетка значков macOS: плитка 824 из 1024
    d.rounded_rectangle((pad, pad, size - pad, size - pad), radius=size * 185 / 1024, fill="#eef1f5")
    k, off = (size - 2 * pad) * 0.8 / 128, pad + (size - 2 * pad) * 0.1
    for x, y, w, h, r, c in RECTS:
        d.rounded_rectangle((off + x * k, off + y * k, off + (x + w) * k, off + (y + h) * k), radius=r * k, fill=c)
    return img


if __name__ == "__main__":
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "packaging/PitchPresenter.icns")
    draw().save(out, sizes=[(16, 16), (32, 32), (64, 64), (128, 128), (256, 256), (512, 512), (1024, 1024)])
    print(out)
