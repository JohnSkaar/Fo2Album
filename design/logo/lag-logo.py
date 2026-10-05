"""Lager Fo2Album-logoen som SVG: symbolet, hele logoen (symbol + «Fo2Album.no») og
appikonet (forenklet). Teksten er gjort om til streker fra Fraunces (SIL OFL 1.1), så filene
ser like ut uten at skriften er installert.

    pip install fonttools brotli
    python design/logo/lag-logo.py

Fargene er primitivene fra design/tokens.css (terrakotta, sand, salvie). I appen tegnes
logoen i src/components/Logo.tsx med semantiske tokens (--color-logo-*).
"""
from pathlib import Path

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

ROOT = Path(__file__).resolve().parents[2]
FONTS = next(ROOT.glob("node_modules/.pnpm/@fontsource-variable+fraunces@*/node_modules/@fontsource-variable/fraunces/files"))
OUT = Path(__file__).resolve().parent

C = {  # design/tokens.css
    "cover": "#9E4529",  # terracotta-700
    "page": "#FFFDF9",  # sand-0
    "page2": "#FAF6F0",  # sand-50
    "line": "#CDBFAB",  # sand-300
    "frame": "#E4D9CA",  # sand-200
    "sky": "#F5C8B3",  # terracotta-200
    "sun": "#B8532F",  # terracotta-600
    "hill": "#6E8F72",  # sage-500
    "heart": "#88A68B",  # sage-400
    "adult": "#B8532F",  # terracotta-600
    "child": "#D9714A",  # terracotta-500
    "ink": "#5C5043",  # sand-700
    "brand": "#B8532F",  # terracotta-600
    "text": "#241E18",  # sand-900
    "muted": "#988A76",  # sand-500
    "bg": "#FAF6F0",  # sand-50
}


def font(name, wght):
    f = TTFont(FONTS / name)
    loc = {a.axisTag: (wght if a.axisTag == "wght" else a.defaultValue) for a in f["fvar"].axes}
    return instancer.instantiateVariableFont(f, loc)


def outline(f, text, size):
    """Teksten som én sti med grunnlinjen i y = 0, og bredden."""
    gs, cmap = f.getGlyphSet(), f.getBestCmap()
    s, x, parts = size / f["head"].unitsPerEm, 0.0, []
    for ch in text:
        g = cmap[ord(ch)]
        pen = SVGPathPen(gs, ntos=lambda v: f"{v:.2f}".rstrip("0").rstrip("."))
        gs[g].draw(TransformPen(pen, (s, 0, 0, -s, x, 0)))
        parts.append(pen.getCommands())
        x += gs[g].width * s
    return " ".join(parts), x


REG = font("fraunces-latin-wght-normal.woff2", 600)
ITA = font("fraunces-latin-wght-italic.woff2", 500)
FOTO, FOTO_W = outline(ITA, "Fotoalbum", 17)


def symbol(gap):
    """Symbolet i 0 0 200 172: familien ser inn i fotoalbumet sammen med oss."""
    fs = 62 / FOTO_W
    return f"""<path d="M100 26c-4.5-5.5-13-3.5-13 3.2 0 5.6 7.6 9.6 13 13.8 5.4-4.2 13-8.2 13-13.8 0-6.7-8.5-8.7-13-3.2z" fill="{C['heart']}"/>
  <path d="M100 60C78 49 46 47 16 54l-6 78c32-6 64-3 90 9 26-12 58-15 90-9l-6-78c-30-7-62-5-84 6z" fill="{C['cover']}"/>
  <path d="M100 63C80 53 51 51 24 57l-5 69c29-5 57-2 81 9z" fill="{C['page']}"/>
  <path d="M100 63c20-10 49-12 76-6l5 69c-29-5-57-2-81 9z" fill="{C['page2']}"/>
  <path d="M100 63v72" stroke="{C['line']}" stroke-width="1.6"/>
  <g transform="rotate(-5 58 86)">
    <rect x="36" y="68" width="45" height="37" rx="1.5" fill="#fff" stroke="{C['frame']}" stroke-width="1.2"/>
    <rect x="40" y="72" width="37" height="25" fill="{C['sky']}"/>
    <circle cx="68" cy="79" r="4" fill="{C['sun']}"/>
    <path d="M40 97v-7c7-7 13-7 19-2 5-5 11-6 18 0v9z" fill="{C['hill']}"/>
  </g>
  <path transform="translate(109 84) rotate(-4) scale({fs:.4f})" d="{FOTO}" fill="{C['ink']}"/>
  <path d="M117 95c13-2 30-2 44-1M119 104c11-1.5 23-1.5 34-1" stroke="{C['line']}" stroke-width="2" stroke-linecap="round" fill="none"/>
  <g stroke="{gap}" stroke-width="3.5">
    <path d="M18 172c0-21 15-33 38-33s38 12 38 33z" fill="{C['adult']}"/>
    <circle cx="56" cy="122" r="18" fill="{C['adult']}"/>
    <path d="M106 172c0-22 16-35 40-35s40 13 40 35z" fill="{C['adult']}"/>
    <circle cx="146" cy="119" r="19" fill="{C['adult']}"/>
    <path d="M74 172c0-15 11-23 26-23s26 8 26 23z" fill="{C['child']}"/>
    <circle cx="100" cy="136" r="13" fill="{C['child']}"/>
  </g>"""


def svg(view, body, label="Fo2Album"):
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="{view}" role="img" aria-label="{label}">
  <title>{label}</title>
  {body}
</svg>
"""


# 1. Symbolet
(OUT / "fo2album-symbol.svg").write_text(svg("0 0 200 172", symbol(C["bg"])))

# 2. Hele logoen: symbolet over «Fo2Album» med en liten «.no»
size = 44
fo2, w1 = outline(REG, "Fo2", size)
alb, w2 = outline(REG, "Album", size)
no, w3 = outline(REG, ".no", size * 0.44)
gap_no = 2
word = w1 + w2 + gap_no + w3
W = max(word, 220) + 40
sym_w = 220
sx = (W - sym_w) / 2
sym_h = 172 * sym_w / 200
base = 20 + sym_h + 26 + size * 0.72
tx = (W - word) / 2
H = base + size * 0.3 + 16
full = f"""<rect width="100%" height="100%" fill="{C['bg']}"/>
  <g transform="translate({sx:.2f} 20) scale({sym_w / 200:.4f})">
  {symbol(C['bg'])}
  </g>
  <g transform="translate({tx:.2f} {base:.2f})">
    <path d="{fo2}" fill="{C['brand']}"/>
    <path transform="translate({w1:.2f} 0)" d="{alb}" fill="{C['text']}"/>
    <path transform="translate({w1 + w2 + gap_no:.2f} 0)" d="{no}" fill="{C['muted']}"/>
  </g>"""
(OUT / "fo2album-logo.svg").write_text(svg(f"0 0 {W:.0f} {H:.0f}", full, "Fo2Album.no"))

# 3. Appikonet: samme bok og familie, forenklet så det kan leses i 16–32 punkter
icon = f"""<defs><clipPath id="flis"><rect x="4" y="4" width="92" height="92" rx="22"/></clipPath></defs>
  <rect x="4" y="4" width="92" height="92" rx="22" fill="{C['brand']}"/>
  <g clip-path="url(#flis)">
    <path d="M50 18c-2.6-3.2-7.6-2-7.6 1.9 0 3.3 4.4 5.6 7.6 8.1 3.2-2.5 7.6-4.8 7.6-8.1 0-3.9-5-5.1-7.6-1.9z" fill="#C7D8C8"/>
    <path d="M50 36C40 30 26 29 14 32l-3 37c14-3 27-1 39 5 12-6 25-8 39-5l-3-37c-12-3-26-2-36 4z" fill="{C['cover']}"/>
    <path d="M50 38C40 33 28 32 18 34l-2.5 31c12-2 24 0 34.5 5z" fill="{C['page']}"/>
    <path d="M50 38c10-5 22-6 32-4l2.5 31c-12-2-24 0-34.5 5z" fill="{C['page2']}"/>
    <rect x="22" y="40" width="21" height="16" rx="1" fill="{C['sky']}" transform="rotate(-5 32 48)"/>
    <path d="M22 56.5v-4c4-4 7.5-4 11-1 3-3 6.5-3.5 10 0v3.4z" fill="{C['hill']}" transform="rotate(-5 32 48)"/>
    <path d="M56 45h21M57 51h18M58 57h13" stroke="{C['line']}" stroke-width="2.6" stroke-linecap="round"/>
    <g stroke="{C['brand']}" stroke-width="2.5">
      <circle cx="37" cy="76" r="10" fill="#7A3520"/>
      <path d="M17 100c0-12 9-18 20-18s20 6 20 18z" fill="#7A3520"/>
      <circle cx="63" cy="83" r="7.5" fill="#F5C8B3"/>
      <path d="M48 100c0-8 6.5-12 15-12s15 4 15 12z" fill="#F5C8B3"/>
    </g>
  </g>"""
(OUT / "fo2album-ikon.svg").write_text(svg("0 0 100 100", icon))

# Stien til «Fotoalbum», til appens Logo.tsx
(ROOT / "apps/desktop/src/components/logoTekst.ts").write_text(
    "// Laget av design/logo/lag-logo.py. Ikke rediger for hånd.\n"
    "/** «Fotoalbum» i Fraunces kursiv, som sti (grunnlinje i y = 0). */\n"
    f'export const FOTOALBUM = "{FOTO}";\n'
    f"export const FOTOALBUM_SKALA = {62 / FOTO_W:.4f};\n"
)
print("Laget:", ", ".join(p.name for p in sorted(OUT.glob("*.svg"))))
