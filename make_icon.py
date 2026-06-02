"""
Generates the pkap app icon: viewfinder design, green + gold colour scheme.
Run with: python3 make_icon.py
Requires: pip3 install Pillow  (already installed)
"""
from PIL import Image, ImageDraw
import numpy as np
import math

SIZE    = 1024
CORNER  = 220   # macOS-style rounded-square corner radius

# ── Colours ───────────────────────────────────────────────────────────────────
GREEN_DARK  = (26,  71,  42, 255)   # #1A472A
GREEN_MID   = (39, 174,  96, 255)   # #27AE60
WHITE_FULL  = (255, 255, 255, 230)
WHITE_DIM   = (255, 255, 255, 100)
GOLD        = (255, 214,   0, 255)  # #FFD600
GOLD_DIM    = (255, 214,   0, 80)

# ── Background gradient (diagonal dark→bright green) ─────────────────────────
def make_gradient():
    y = np.linspace(0, 1, SIZE).reshape(SIZE, 1)
    x = np.linspace(0, 1, SIZE).reshape(1, SIZE)
    t = np.clip(x * 0.4 + y * 0.6, 0, 1)[:, :, np.newaxis]
    dark  = np.array(GREEN_DARK[:3],  dtype=np.float32)
    light = np.array(GREEN_MID[:3],   dtype=np.float32)
    rgb   = (dark * (1 - t) + light * t).astype(np.uint8)
    alpha = np.full((SIZE, SIZE, 1), 255, dtype=np.uint8)
    return Image.fromarray(np.concatenate([rgb, alpha], axis=2), 'RGBA')

bg   = make_gradient()

# Rounded-rectangle mask (transparent corners)
mask = Image.new('L', (SIZE, SIZE), 0)
ImageDraw.Draw(mask).rounded_rectangle([0, 0, SIZE-1, SIZE-1], radius=CORNER, fill=255)
img  = Image.new('RGBA', (SIZE, SIZE), (0, 0, 0, 0))
img.paste(bg, mask=mask)

draw = ImageDraw.Draw(img, 'RGBA')
C    = SIZE // 2   # centre

# ── Viewfinder crosshair arms (white) ─────────────────────────────────────────
AW   = 22     # arm width
GAP  = 260    # start of arm from edge
STOP = 430    # end of arm (gap around centre circle)

for (x0, y0, x1, y1) in [
    (GAP, C - AW//2,        STOP,        C + AW//2),         # left
    (SIZE-STOP, C - AW//2,  SIZE-GAP,    C + AW//2),         # right
    (C - AW//2, GAP,        C + AW//2,   STOP),              # top
    (C - AW//2, SIZE-STOP,  C + AW//2,   SIZE-GAP),          # bottom
]:
    draw.rounded_rectangle([x0, y0, x1, y1], radius=10, fill=WHITE_FULL)

# ── Corner brackets (gold L-shapes) ──────────────────────────────────────────
T  = 30    # bracket thickness
L  = 155   # bracket arm length
M  = 148   # margin from edge

def bracket(ox, oy, dx, dy):
    """L-bracket anchored at (ox, oy), growing in direction (dx, dy)."""
    # horizontal bar
    hx0, hx1 = sorted([ox, ox + dx * L])
    hy0, hy1 = sorted([oy, oy + dy * T])
    draw.rounded_rectangle([hx0, hy0, hx1, hy1], radius=T//2, fill=GOLD)
    # vertical bar
    vx0, vx1 = sorted([ox, ox + dx * T])
    vy0, vy1 = sorted([oy, oy + dy * L])
    draw.rounded_rectangle([vx0, vy0, vx1, vy1], radius=T//2, fill=GOLD)

bracket(M,          M,          +1, +1)   # top-left
bracket(SIZE - M,   M,          -1, +1)   # top-right
bracket(M,          SIZE - M,   +1, -1)   # bottom-left
bracket(SIZE - M,   SIZE - M,   -1, -1)   # bottom-right

# ── Gold glow rings behind record button ──────────────────────────────────────
for r, alpha in [(150, 35), (120, 55)]:
    glow = Image.new('RGBA', (SIZE, SIZE), (0, 0, 0, 0))
    gd   = ImageDraw.Draw(glow, 'RGBA')
    gd.ellipse([C-r, C-r, C+r, C+r], fill=(255, 214, 0, alpha))
    img  = Image.alpha_composite(img, glow)
    draw = ImageDraw.Draw(img, 'RGBA')

# ── Record button (gold filled circle with white ring) ────────────────────────
RO = 95    # outer ring radius
RI = 66    # inner fill radius

draw.ellipse([C-RO, C-RO, C+RO, C+RO], outline=WHITE_FULL, width=18)
draw.ellipse([C-RI, C-RI, C+RI, C+RI], fill=GOLD)

# shine highlight on gold button
draw.ellipse([C-38, C-50, C+14, C-12], fill=(255, 255, 255, 110))

# ── Save ──────────────────────────────────────────────────────────────────────
OUT = 'src-tauri/icons/app-icon.png'
img.save(OUT, 'PNG')
print(f"Saved {OUT}  ({SIZE}×{SIZE} px)")
