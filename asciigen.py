#!/usr/bin/env python3
"""
ASCII Art Text Renderer

Renders text as ASCII art using 5-level block/shade characters: █▓▒░ and space.

Combined brightness x position shading:
    density = pixel_brightness * (0.25 + gradient_factor * 0.75)

With --direction none (default), pure brightness mapping.
With a directional sweep, the dense end keeps full brightness while the light
end is attenuated to 25%, producing a gradient that still follows letterforms.

Usage:
  python asciigen.py "RAPS"
  python asciigen.py "RAPS" --width 40 --style italic --color yellow
  python asciigen.py "RAPS" --direction center-out --color '#FF8800'
  python asciigen.py "RAPS" --color-mix horizontal --palette fire
  python asciigen.py "RAPS" --color-mix rainbow -w 60
  python asciigen.py "RAPS" --color-mix shade --palette "red,yellow,white"
  python asciigen.py "RAPS" --showcase
"""

import argparse
import colorsys
import os
import sys

from PIL import Image, ImageDraw, ImageFont

# ── 5-level charset ────────────────────────────────────────────────────────

#   █ = full block   (densest)
#   ▓ = dark shade
#   ▒ = medium shade
#   ░ = light shade   (lightest visible)
#   ' ' = empty       (below threshold)
SHADES = "█▓▒░"

# Render at 2x resolution then LANCZOS-downscale for smooth antialiased edges.
SUPERSAMPLE = 2

# Terminal cell aspect ratio: width x height in source pixels.
# Most terminal fonts are ~1:2 (w:h), so 2px wide x 4px tall preserves shape.
CELL_W = 2
CELL_H = 4

# ── Font map by style ──────────────────────────────────────────────────────

FONT_MAP = {
    "regular": [
        # Windows
        "C:/Windows/Fonts/consola.ttf",       # Consolas
        "C:/Windows/Fonts/cour.ttf",          # Courier New
        "C:/Windows/Fonts/arial.ttf",
        # Linux
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
        "/usr/share/fonts/truetype/freefont/FreeMono.ttf",
        # macOS
        "/Library/Fonts/Courier New.ttf",
        "/System/Library/Fonts/Monaco.ttf",
    ],
    "bold": [
        "C:/Windows/Fonts/consolab.ttf",      # Consolas Bold
        "C:/Windows/Fonts/courbd.ttf",        # Courier New Bold
        "C:/Windows/Fonts/arialbd.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationMono-Bold.ttf",
        "/usr/share/fonts/truetype/freefont/FreeMonoBold.ttf",
        "/Library/Fonts/Courier New Bold.ttf",
    ],
    "italic": [
        "C:/Windows/Fonts/consolai.ttf",      # Consolas Italic
        "C:/Windows/Fonts/couri.ttf",         # Courier New Italic
        "C:/Windows/Fonts/ariali.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Oblique.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationMono-Italic.ttf",
        "/usr/share/fonts/truetype/freefont/FreeMonoOblique.ttf",
        "/Library/Fonts/Courier New Italic.ttf",
    ],
    "bold-italic": [
        "C:/Windows/Fonts/consolaz.ttf",      # Consolas Bold Italic
        "C:/Windows/Fonts/courbi.ttf",        # Courier New Bold Italic
        "C:/Windows/Fonts/arialbi.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-BoldOblique.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationMono-BoldItalic.ttf",
        "/usr/share/fonts/truetype/freefont/FreeMonoBoldOblique.ttf",
        "/Library/Fonts/Courier New Bold Italic.ttf",
    ],
}

STYLES = list(FONT_MAP.keys())

# ── ANSI color codes ───────────────────────────────────────────────────────

ANSI_COLORS = {
    "black":          "\033[30m",
    "red":            "\033[31m",
    "green":          "\033[32m",
    "yellow":         "\033[33m",
    "blue":           "\033[34m",
    "magenta":        "\033[35m",
    "cyan":           "\033[36m",
    "white":          "\033[37m",
    "bright-black":   "\033[90m",
    "bright-red":     "\033[91m",
    "bright-green":   "\033[92m",
    "bright-yellow":  "\033[93m",
    "bright-blue":    "\033[94m",
    "bright-magenta": "\033[95m",
    "bright-cyan":    "\033[96m",
    "bright-white":   "\033[97m",
}

ANSI_RESET = "\033[0m"

COLOR_NAMES = list(ANSI_COLORS.keys())

# ── Color palettes for mixed coloring ─────────────────────────────────────

COLOR_PALETTES = {
    "fire":    ["#CC0000", "#FF4400", "#FF8800", "#FFCC00", "#FFFF44"],
    "ice":     ["#0000CC", "#0066FF", "#00BBFF", "#00EEFF", "#AAFFFF"],
    "ocean":   ["#001166", "#0044AA", "#0088CC", "#00BBDD", "#66DDEE"],
    "neon":    ["#FF00FF", "#FF00AA", "#AA00FF", "#5500FF", "#00FFFF"],
    "sunset":  ["#FF0044", "#FF3300", "#FF6600", "#FF9900", "#FFCC00"],
    "forest":  ["#004400", "#006600", "#228B22", "#44AA00", "#88CC00"],
    "gold":    ["#8B6914", "#B8860B", "#DAA520", "#FFD700", "#FFEE66"],
    "rainbow": ["#FF0000", "#FF8800", "#FFFF00", "#00FF00", "#0088FF", "#8800FF"],
    "matrix":  ["#003300", "#006600", "#00AA00", "#00FF00", "#66FF66"],
    "lava":    ["#330000", "#660000", "#CC0000", "#FF4400", "#FFAA00"],
}

PALETTE_NAMES = list(COLOR_PALETTES.keys())

MIX_MODES = ["horizontal", "vertical", "diagonal", "rainbow", "shade", "letter"]


# ── Font loading ───────────────────────────────────────────────────────────

def get_font(size, style="bold"):
    """Load the first available TrueType font at the given pixel size and style."""
    candidates = FONT_MAP.get(style, FONT_MAP["bold"])
    for path in candidates:
        if os.path.exists(path):
            try:
                return ImageFont.truetype(path, size)
            except Exception:
                continue
    # Fallback: try bold if requested style unavailable
    if style != "bold":
        for path in FONT_MAP["bold"]:
            if os.path.exists(path):
                try:
                    return ImageFont.truetype(path, size)
                except Exception:
                    continue
    try:
        return ImageFont.load_default(size=size)  # Pillow >= 10
    except TypeError:
        return ImageFont.load_default()


# ── Color helpers ──────────────────────────────────────────────────────────

def _color_code(color):
    """Return ANSI escape for a named color or #RRGGBB hex string."""
    if not color:
        return None
    if color in ANSI_COLORS:
        return ANSI_COLORS[color]
    if color.startswith("#") and len(color) == 7:
        try:
            r = int(color[1:3], 16)
            g = int(color[3:5], 16)
            b = int(color[5:7], 16)
            return f"\033[38;2;{r};{g};{b}m"
        except ValueError:
            return None
    return None


def colorize(text, color):
    """Wrap each line of text in ANSI color escapes. Returns text unchanged if
    color is None/empty or unrecognized."""
    code = _color_code(color)
    if not code:
        return text
    return "\n".join(
        f"{code}{line}{ANSI_RESET}" if line.strip() else line
        for line in text.split("\n")
    )


# ── Per-cell color mixing ─────────────────────────────────────────────────

_NAMED_RGB = {
    "black":          (0, 0, 0),
    "red":            (205, 0, 0),
    "green":          (0, 205, 0),
    "yellow":         (205, 205, 0),
    "blue":           (0, 0, 238),
    "magenta":        (205, 0, 205),
    "cyan":           (0, 205, 205),
    "white":          (229, 229, 229),
    "bright-black":   (127, 127, 127),
    "bright-red":     (255, 0, 0),
    "bright-green":   (0, 255, 0),
    "bright-yellow":  (255, 255, 0),
    "bright-blue":    (92, 92, 255),
    "bright-magenta": (255, 0, 255),
    "bright-cyan":    (0, 255, 255),
    "bright-white":   (255, 255, 255),
}


def _to_rgb(color):
    """Convert a color name or #RRGGBB hex string to (R, G, B) tuple."""
    if color.startswith("#") and len(color) == 7:
        return int(color[1:3], 16), int(color[3:5], 16), int(color[5:7], 16)
    return _NAMED_RGB.get(color, (255, 255, 255))


def _interpolate_palette(rgb_palette, t):
    """Smoothly interpolate between palette colors at position t (0.0–1.0)."""
    t = max(0.0, min(1.0, t))
    n = len(rgb_palette)
    if n == 1:
        return rgb_palette[0]
    scaled = t * (n - 1)
    idx = int(scaled)
    frac = scaled - idx
    if idx >= n - 1:
        return rgb_palette[-1]
    r1, g1, b1 = rgb_palette[idx]
    r2, g2, b2 = rgb_palette[idx + 1]
    return int(r1 + (r2 - r1) * frac), int(g1 + (g2 - g1) * frac), int(b1 + (b2 - b1) * frac)


def _resolve_palette(palette):
    """Resolve a palette name or comma-separated color list to RGB tuples."""
    if palette in COLOR_PALETTES:
        return [_to_rgb(c) for c in COLOR_PALETTES[palette]]
    colors = [c.strip() for c in palette.split(",")]
    rgbs = [_to_rgb(c) for c in colors if c]
    return rgbs if rgbs else [_to_rgb("white")]


def colorize_mixed(text, mode, rgb_palette, num_letters=4):
    """Apply per-cell coloring based on mode and palette.

    Modes:
      horizontal — left→right gradient across columns
      vertical   — top→bottom gradient across rows
      diagonal   — top-left→bottom-right gradient
      rainbow    — full HSV spectrum sweep across columns
      shade      — map each shade character (█▓▒░) to palette position
      letter     — divide columns into letter-sized bands, each band a palette color
    """
    lines = text.split("\n")
    if not lines or not rgb_palette:
        return text

    max_cols = max((len(line) for line in lines), default=1)
    max_rows = len(lines)

    result = []
    for row_idx, line in enumerate(lines):
        if not line.strip():
            result.append(line)
            continue
        colored_chars = []
        for col_idx, ch in enumerate(line):
            if ch == " ":
                colored_chars.append(ch)
                continue

            if mode == "shade":
                si = SHADES.index(ch) if ch in SHADES else 0
                t = si / max(len(SHADES) - 1, 1)
                r, g, b = _interpolate_palette(rgb_palette, t)
            elif mode == "letter":
                band = col_idx * num_letters // max(max_cols, 1)
                band = min(band, num_letters - 1)
                ci = band % len(rgb_palette)
                r, g, b = rgb_palette[ci]
            elif mode == "horizontal":
                t = col_idx / max(max_cols - 1, 1)
                r, g, b = _interpolate_palette(rgb_palette, t)
            elif mode == "vertical":
                t = row_idx / max(max_rows - 1, 1)
                r, g, b = _interpolate_palette(rgb_palette, t)
            elif mode == "diagonal":
                tx = col_idx / max(max_cols - 1, 1)
                ty = row_idx / max(max_rows - 1, 1)
                t = (tx + ty) / 2
                r, g, b = _interpolate_palette(rgb_palette, t)
            elif mode == "rainbow":
                t = col_idx / max(max_cols - 1, 1)
                r_f, g_f, b_f = colorsys.hsv_to_rgb(t, 1.0, 1.0)
                r, g, b = int(r_f * 255), int(g_f * 255), int(b_f * 255)
            else:
                r, g, b = rgb_palette[0]

            colored_chars.append(f"\033[38;2;{r};{g};{b}m{ch}")

        result.append("".join(colored_chars) + ANSI_RESET)

    return "\n".join(result)


# ── Bitmap rendering ──────────────────────────────────────────────────────

def _measure_text(font, text):
    """Return (width, height) of rendered text via the best available Pillow API."""
    try:
        bbox = font.getbbox(text)
        return bbox[2] - bbox[0], bbox[3] - bbox[1]
    except AttributeError:
        return font.getsize(text)


def render_bitmap(text, font_size, style="bold"):
    """Render text to a tightly-cropped grayscale bitmap with supersampled antialiasing."""
    hi_size = font_size * SUPERSAMPLE
    font = get_font(hi_size, style)

    # Measure to allocate a right-sized canvas at high resolution
    tw, th = _measure_text(font, text)
    pad = max(4, hi_size // 8)
    canvas_w = tw + pad * 2
    canvas_h = th + pad * 2

    img = Image.new("L", (canvas_w, canvas_h), 0)
    draw = ImageDraw.Draw(img)
    draw.text((pad, pad), text, fill=255, font=font)

    bbox = img.getbbox()
    if bbox is None:
        return img

    # Tight crop with small margin to avoid clipping
    margin = 2 * SUPERSAMPLE
    img = img.crop((
        max(0, bbox[0] - margin),
        max(0, bbox[1] - margin),
        min(canvas_w, bbox[2] + margin),
        min(canvas_h, bbox[3] + margin),
    ))

    # Downsample with LANCZOS for smooth antialiased edges
    target_w = max(1, img.size[0] // SUPERSAMPLE)
    target_h = max(1, img.size[1] // SUPERSAMPLE)
    return img.resize((target_w, target_h), Image.LANCZOS)


# ── ASCII conversion ──────────────────────────────────────────────────────

def bitmap_to_ascii(img, direction="none", threshold=0.08, gamma=1.0,
                    bright_core=0.0):
    """Convert grayscale bitmap to 5-level ASCII art.

    Levels:  █ (dense) → ▓ → ▒ → ░ (light) → ' ' (empty)

    Combined brightness x position shading:
        density = brightness * (0.25 + gradient_factor * 0.75)

    With direction="none", gradient_factor=1.0 everywhere → pure brightness.
    With a directional sweep, the dense end stays at full brightness while the
    light end is attenuated to 25%, creating a gradient that still follows the
    letterform shape.

    bright_core: when > 0, any cell with brightness >= this value is forced to █
                 regardless of gradient or gamma.  Keeps the interior of letters
                 always fully visible.  Typical values: 0.4–0.7.
    """
    W, H = img.size
    pixels = img.load()

    cols = W // CELL_W
    rows = H // CELL_H

    lines = []
    for row in range(rows):
        chars = []
        for col in range(cols):
            x0, y0 = col * CELL_W, row * CELL_H

            # Average brightness of this cell (0.0 – 1.0)
            total = 0
            count = 0
            for dy in range(CELL_H):
                for dx in range(CELL_W):
                    px, py = x0 + dx, y0 + dy
                    if 0 <= px < W and 0 <= py < H:
                        total += pixels[px, py]
                        count += 1

            brightness = total / (count * 255) if count else 0.0

            if brightness < threshold:
                chars.append(" ")
            elif bright_core > 0 and brightness >= bright_core:
                # Core pixel — always fully dense
                chars.append(SHADES[0])
            else:
                gf = _gradient_factor(col, cols, direction)
                density = brightness * (0.25 + gf * 0.75)
                # Normalize above threshold into 0.0–1.0
                norm = (density - threshold) / (1.0 - threshold)
                norm = max(0.0, min(1.0, norm))
                # Apply gamma for perceptual tuning
                if gamma != 1.0:
                    norm = norm ** (1.0 / gamma)
                # Map to 5 levels: high density → █, low → ░
                idx = round((1.0 - norm) * (len(SHADES) - 1))
                chars.append(SHADES[idx])

        lines.append("".join(chars).rstrip())

    # Strip leading/trailing blank lines
    while lines and not lines[0].strip():
        lines.pop(0)
    while lines and not lines[-1].strip():
        lines.pop()

    return "\n".join(lines)


def _gradient_factor(col, total_cols, direction):
    """Return 0.0–1.0 positional factor (1.0 = dense end, 0.0 = light end)."""
    if direction == "none":
        return 1.0
    t = col / max(total_cols - 1, 1)
    if direction == "right-left":
        t = 1 - t
    elif direction == "center-out":
        t = abs(t - 0.5) * 2
    elif direction == "center-in":
        t = 1 - abs(t - 0.5) * 2
    return 1.0 - t


# ── Public API ─────────────────────────────────────────────────────────────

def auto_font_size(text, target_cols, style="bold"):
    """Binary search for a font size that produces ~target_cols output columns."""
    lo, hi = 8, 400
    best = 64
    for _ in range(20):
        mid = (lo + hi) // 2
        img = render_bitmap(text, mid, style)
        cols = img.size[0] // CELL_W
        if cols < target_cols:
            lo = mid + 1
        else:
            best = mid
            hi = mid - 1
        if lo > hi:
            break
    return best


def render(text, font_size=64, width=None, direction="none", gamma=1.0,
           bright_core=0.0, style="bold", color=None, color_mix=None,
           palette=None):
    """Render text as ASCII art string, optionally colorized.

    color      — uniform ANSI color for the whole output
    color_mix  — per-cell mixing mode (horizontal, vertical, diagonal,
                 rainbow, shade, letter)
    palette    — palette name or comma-separated colors for color_mix
    """
    if width:
        font_size = auto_font_size(text, width, style)
    img = render_bitmap(text, font_size, style)
    art = bitmap_to_ascii(img, direction=direction, gamma=gamma,
                          bright_core=bright_core)
    if color_mix:
        pal_rgb = _resolve_palette(palette or "fire")
        art = colorize_mixed(art, color_mix, pal_rgb, num_letters=len(text))
    elif color:
        art = colorize(art, color)
    return art


# ── Showcase ───────────────────────────────────────────────────────────────

def _run_showcase(text):
    """Print 40+ variant combinations grouped into sections."""
    DIRECTIONS = ["none", "left-right", "right-left", "center-out", "center-in"]
    W = 45  # standard showcase width
    n = 0

    def heading(title):
        bar = "\u2550" * 60
        print(f"\n{bar}")
        print(f"  {title}")
        print(bar)

    def variant(label, **kw):
        nonlocal n
        n += 1
        print(f"\n [{n:02d}] {label}")
        print(f"      {'\u2500' * 50}")
        print(render(text, **kw))

    # ── Section 1: Direction sweep ──────────────────────────────
    heading("SECTION 1 \u2014 Direction sweep  (width=45, gamma=1.0)")
    for d in DIRECTIONS:
        variant(f"direction={d}", width=W, direction=d)

    # ── Section 2: Gamma sweep ──────────────────────────────────
    heading("SECTION 2 \u2014 Gamma sweep  (width=45, direction=none)")
    for g in [0.4, 0.6, 0.8, 1.0, 1.2, 1.5, 2.0]:
        variant(f"gamma={g}", width=W, gamma=g)

    # ── Section 3: Gamma × direction ────────────────────────────
    heading("SECTION 3 \u2014 Gamma \u00d7 direction  (width=45)")
    for d, g in [("left-right", 0.6), ("left-right", 1.5),
                 ("right-left", 0.6), ("right-left", 1.5),
                 ("center-out", 0.6), ("center-out", 1.5),
                 ("center-in",  0.6), ("center-in",  1.5)]:
        variant(f"direction={d}  gamma={g}", width=W, direction=d, gamma=g)

    # ── Section 4: Width scaling ────────────────────────────────
    heading("SECTION 4 \u2014 Width scaling  (direction=none, gamma=1.0)")
    for w in [20, 35, 50, 70]:
        variant(f"width={w}", width=w)

    # ── Section 5: Width × direction ────────────────────────────
    heading("SECTION 5 \u2014 Width \u00d7 direction  (gamma=1.0)")
    for w, d in [(25, "center-out"), (25, "left-right"),
                 (60, "center-out"), (60, "left-right")]:
        variant(f"width={w}  direction={d}", width=w, direction=d)

    # ── Section 6: Font size ────────────────────────────────────
    heading("SECTION 6 \u2014 Font size  (direction=center-in, gamma=1.2)")
    for s in [32, 48, 80]:
        variant(f"size={s}px", font_size=s, direction="center-in", gamma=1.2)

    # ── Section 7: Bright core ──────────────────────────────────
    heading("SECTION 7 \u2014 Bright core  (width=45)")
    for bc in [0.3, 0.5, 0.7]:
        variant(f"bright-core={bc}  direction=left-right",
                width=W, direction="left-right", bright_core=bc)
    for d in ["center-out", "right-left"]:
        variant(f"bright-core=0  direction={d}  (no core)",
                width=W, direction=d)
        variant(f"bright-core=0.5  direction={d}",
                width=W, direction=d, bright_core=0.5)

    # ── Section 8: Font styles ──────────────────────────────────
    heading("SECTION 8 \u2014 Font styles  (width=45, direction=none)")
    for st in STYLES:
        variant(f"style={st}", width=W, style=st)

    # ── Section 9: Font style × direction ───────────────────────
    heading("SECTION 9 \u2014 Font style \u00d7 direction  (width=45)")
    for st, d in [("italic", "left-right"), ("italic", "center-out"),
                  ("bold-italic", "left-right"), ("regular", "center-in")]:
        variant(f"style={st}  direction={d}", width=W, style=st, direction=d)

    # ── Section 10: Colors ──────────────────────────────────────
    heading("SECTION 10 \u2014 Colors  (width=45, direction=none)")
    for c in ["red", "green", "yellow", "blue", "magenta", "cyan",
              "bright-yellow", "bright-cyan"]:
        variant(f"color={c}", width=W, color=c)

    # ── Section 11: Color × direction × style ───────────────────
    heading("SECTION 11 \u2014 Color \u00d7 direction \u00d7 style  (width=45)")
    for c, d, st in [("yellow",         "left-right",  "bold"),
                     ("#FF8800",        "center-out",  "italic"),
                     ("bright-cyan",    "right-left",  "bold-italic"),
                     ("#FF3366",        "center-in",   "regular"),
                     ("bright-green",   "left-right",  "bold"),
                     ("#9966FF",        "center-out",  "italic")]:
        variant(f"color={c}  direction={d}  style={st}",
                width=W, color=c, direction=d, style=st)

    # ── Section 12: Color mix modes ──────────────────────────────
    heading("SECTION 12 \u2014 Color mix modes  (width=45, palette=fire)")
    for mode in MIX_MODES:
        pal = "rainbow" if mode == "letter" else "fire"
        variant(f"color-mix={mode}  palette={pal}",
                width=W, color_mix=mode, palette=pal)

    # ── Section 13: Palette showcase ──────────────────────────────
    heading("SECTION 13 \u2014 Palettes  (width=45, color-mix=horizontal)")
    for pal in PALETTE_NAMES:
        variant(f"palette={pal}", width=W, color_mix="horizontal", palette=pal)

    # ── Section 14: Color mix × direction × style ─────────────────
    heading("SECTION 14 \u2014 Color mix \u00d7 direction \u00d7 style  (width=45)")
    for mix, pal, d, st in [
            ("horizontal", "sunset",  "left-right",  "bold"),
            ("vertical",   "ice",     "center-out",  "italic"),
            ("diagonal",   "neon",    "none",         "bold-italic"),
            ("rainbow",    "rainbow", "center-in",    "regular"),
            ("shade",      "gold",    "left-right",   "bold"),
            ("letter",     "rainbow", "none",          "bold")]:
        variant(f"mix={mix}  pal={pal}  dir={d}  style={st}",
                width=W, color_mix=mix, palette=pal, direction=d, style=st)

    print(f"\n{'\u2550' * 60}")
    print(f"  TOTAL: {n} variants")
    print(f"{'\u2550' * 60}\n")


# ── CLI ────────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="ASCII art text renderer with block/shade characters"
    )
    parser.add_argument("text", nargs="?", default="RAPS")
    parser.add_argument("--size", "-s", type=int, default=64,
                        help="font size in pixels (default: 64)")
    parser.add_argument("--width", "-w", type=int, default=None,
                        help="target width in terminal columns (auto-scales font)")
    parser.add_argument("--style", type=str,
                        choices=STYLES, default="bold",
                        help="font style (default: bold)")
    parser.add_argument("--color", "-c", type=str, default=None,
                        help="uniform ANSI color: name (red, yellow, ...) "
                             "or hex (#FF8800)")
    parser.add_argument("--color-mix", "-m", type=str,
                        choices=MIX_MODES, default=None,
                        help="per-cell color mixing mode")
    parser.add_argument("--palette", "-p", type=str, default=None,
                        help="palette for --color-mix: name (fire, ice, neon, "
                             "sunset, ...) or comma-separated colors "
                             "(\"red,yellow,white\")")
    parser.add_argument("--direction", "-d",
                        choices=["none", "left-right", "right-left",
                                 "center-out", "center-in"],
                        default="none",
                        help="gradient sweep direction (default: none)")
    parser.add_argument("--gamma", "-g", type=float, default=1.0,
                        help="shade curve: >1 = denser, <1 = lighter (default: 1.0)")
    parser.add_argument("--bright-core", "-b", type=float, nargs="?",
                        const=0.5, default=0.0,
                        help="force cells above this brightness to \u2588 "
                             "(default when flag used: 0.5)")
    parser.add_argument("--demo", action="store_true",
                        help="render all 5 directions")
    parser.add_argument("--showcase", action="store_true",
                        help="render 50+ variant combinations")
    parser.add_argument("--fonts", action="store_true",
                        help="list detected fonts by style and exit")
    parser.add_argument("--colors", action="store_true",
                        help="list available color names and exit")
    args = parser.parse_args()

    if args.fonts:
        for st in STYLES:
            print(f"\n  [{st}]")
            for p in FONT_MAP[st]:
                status = "\u2713" if os.path.exists(p) else "\u2717"
                print(f"    {status} {p}")
        print()
        sys.exit(0)

    if args.colors:
        print("\nNamed colors:")
        for name in COLOR_NAMES:
            code = ANSI_COLORS[name]
            print(f"  {code}\u2588\u2588 {name}{ANSI_RESET}")
        print(f"\nHex RGB:  --color '#FF8800'")
        print(f"True color uses 24-bit escapes (\\033[38;2;R;G;Bm)")
        print(f"\nPalettes (for --color-mix):")
        for pname in PALETTE_NAMES:
            swatches = ""
            for c in COLOR_PALETTES[pname]:
                r, g, b = _to_rgb(c)
                swatches += f"\033[38;2;{r};{g};{b}m\u2588\u2588"
            print(f"  {swatches}{ANSI_RESET}  {pname}")
        print(f"\nCustom:  --palette 'red,yellow,#00FF00'\n")
        sys.exit(0)

    text = args.text.replace("\\n", "\n")
    bc = args.bright_core

    if args.showcase:
        _run_showcase(text)
    elif args.demo:
        for d in ["none", "left-right", "right-left", "center-out", "center-in"]:
            print(f"\u2500\u2500 {d} {'\u2500' * 40}")
            print(render(text, args.size, args.width, direction=d,
                         gamma=args.gamma, bright_core=bc,
                         style=args.style, color=args.color,
                         color_mix=args.color_mix, palette=args.palette))
            print()
    else:
        print(render(text, args.size, args.width, args.direction,
                     args.gamma, bc, args.style, args.color,
                     args.color_mix, args.palette))
