"""Render LiquidVoice frost-pill icon PNGs + ICO for Tauri."""
from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
ICONS = ROOT / "src-tauri" / "icons"
OUT_SRC = ROOT / "tools" / "liquidvoice-icon-1024.png"

DISC = (197, 202, 214, 255)  # #c5cad6
RED = (239, 68, 68, 255)  # #ef4444


def lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def gradient_color(t: float) -> tuple[int, int, int, int]:
    # #4a5068 -> #1c1f2c
    r = int(lerp(0x4A, 0x1C, t))
    g = int(lerp(0x50, 0x1F, t))
    b = int(lerp(0x68, 0x2C, t))
    return (r, g, b, 255)


def rounded_rect_mask(size: tuple[int, int], box: tuple[float, float, float, float], radius: float) -> Image.Image:
    mask = Image.new("L", size, 0)
    draw = ImageDraw.Draw(mask)
    draw.rounded_rectangle(box, radius=radius, fill=255)
    return mask


def render(size: int) -> Image.Image:
    s = size / 128.0
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))

    # Gray disc
    disc = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(disc)
    cx = cy = 64 * s
    r = 52 * s
    d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=DISC)
    img = Image.alpha_composite(img, disc)

    # Pill geometry (viewBox units)
    px, py, pw, ph = 28 * s, 52 * s, 72 * s, 24 * s
    pr = 12 * s
    box = (px, py, px + pw, py + ph)

    # Soft outline shadow under pill
    shadow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    sd = ImageDraw.Draw(shadow)
    sd.rounded_rectangle(
        (px, py + 1.5 * s, px + pw, py + ph + 1.5 * s),
        radius=pr,
        fill=(0, 0, 0, 90),
    )
    shadow = shadow.filter(ImageFilter.GaussianBlur(radius=max(1.0, 2.2 * s)))
    img = Image.alpha_composite(img, shadow)

    # Gradient pill body
    pill = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    mask = rounded_rect_mask((size, size), box, pr)
    # diagonal-ish left→right darkening
    for x in range(size):
        t = (x - px) / max(pw, 1)
        t = 0.0 if t < 0 else 1.0 if t > 1 else t
        color = gradient_color(t)
        col = Image.new("RGBA", (1, size), color)
        pill.paste(col, (x, 0))
    pill.putalpha(mask)
    img = Image.alpha_composite(img, pill)

    # Dark rim + light inner edge
    rim = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    rd = ImageDraw.Draw(rim)
    rd.rounded_rectangle(box, radius=pr, outline=(0, 0, 0, 56), width=max(1, int(round(s))))
    inset = 0.75 * s
    rd.rounded_rectangle(
        (px + inset, py + inset, px + pw - inset, py + ph - inset),
        radius=max(1, pr - inset),
        outline=(255, 255, 255, 36),
        width=max(1, int(round(0.75 * s))),
    )
    img = Image.alpha_composite(img, rim)

    # Waveform bars (shorter height)
    bars = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    bd = ImageDraw.Draw(bars)
    specs = [
        (38, 61.5, 2.6, 5),
        (43.2, 59.5, 2.6, 9),
        (48.4, 58, 2.6, 12),
        (53.6, 59.5, 2.6, 9),
        (58.8, 60.5, 2.6, 7),
        (64, 58.5, 2.6, 11),
        (69.2, 60, 2.6, 8),
    ]
    fill = (255, 255, 255, int(255 * 0.9))
    for x, y, w, h in specs:
        bx, by, bw, bh = x * s, y * s, w * s, h * s
        br = (w * s) / 2
        bd.rounded_rectangle((bx, by, bx + bw, by + bh), radius=br, fill=fill)
    img = Image.alpha_composite(img, bars)

    # Red live dot
    dot = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    dd = ImageDraw.Draw(dot)
    dx, dy, dr = 90 * s, 64 * s, 4 * s
    dd.ellipse((dx - dr, dy - dr, dx + dr, dy + dr), fill=RED)
    img = Image.alpha_composite(img, dot)

    return img


def main() -> None:
    ICONS.mkdir(parents=True, exist_ok=True)

    master = render(1024)
    master.save(OUT_SRC, "PNG")
    print(f"wrote {OUT_SRC}")

    # Core Tauri / Windows sizes
    sizes = {
        "32x32.png": 32,
        "128x128.png": 128,
        "128x128@2x.png": 256,
        "icon.png": 512,
        "StoreLogo.png": 50,
        "Square30x30Logo.png": 30,
        "Square44x44Logo.png": 44,
        "Square71x71Logo.png": 71,
        "Square89x89Logo.png": 89,
        "Square107x107Logo.png": 107,
        "Square142x142Logo.png": 142,
        "Square150x150Logo.png": 150,
        "Square284x284Logo.png": 284,
        "Square310x310Logo.png": 310,
    }
    for name, px in sizes.items():
        render(px).save(ICONS / name, "PNG")
        print(f"wrote {ICONS / name}")

    # Multi-size ICO for EXE / tray
    ico_sizes = [16, 24, 32, 48, 64, 128, 256]
    ico_images = [render(px) for px in ico_sizes]
    ico_path = ICONS / "icon.ico"
    ico_images[-1].save(
        ico_path,
        format="ICO",
        append_images=ico_images[:-1],
        sizes=[(px, px) for px in ico_sizes],
    )
    print(f"wrote {ico_path}")


if __name__ == "__main__":
    main()
