#!/usr/bin/env python3
"""Genera las imágenes del README de Solar (docs/readme/*.svg).

Todo es vectorial y el texto va convertido en trazos, así se ve igual en
cualquier navegador sin depender de las fuentes instaladas.

Necesita fontTools (pip install fonttools), los íconos Phosphor (los trae el
crate egui-phosphor en ~/.cargo/registry) y la fuente Inter.

    python3 docs/readme/generar.py
"""

import glob
import os
import re
import sys

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.boundsPen import BoundsPen
from fontTools.ttLib import TTFont

AQUI = os.path.dirname(os.path.abspath(__file__))


def buscar(patrones):
    for p in patrones:
        r = sorted(glob.glob(os.path.expanduser(p)))
        if r:
            return r[-1]
    sys.exit(f"No encontré: {patrones[0]}")


PHOSPHOR = buscar(["~/.cargo/registry/src/*/egui-phosphor-0.14*/res/Phosphor-Fill.ttf"])
PHOSPHOR_LINEA = buscar(["~/.cargo/registry/src/*/egui-phosphor-0.14*/res/Phosphor.ttf"])
CODIGOS = buscar(["~/.cargo/registry/src/*/egui-phosphor-0.14*/src/variants/codepoints.rs"])
INTER_NEGRITA = buscar(["/usr/share/fonts/opentype/inter/Inter-Bold.otf", "/usr/share/fonts/*/Inter-Bold.*"])
INTER_MEDIA = buscar(["/usr/share/fonts/opentype/inter/Inter-Medium.otf", "/usr/share/fonts/*/Inter-Medium.*"])
INTER = buscar(["/usr/share/fonts/opentype/inter/Inter-Regular.otf", "/usr/share/fonts/*/Inter-Regular.*"])

CODIGO = {
    m.group(1): int(m.group(2), 16)
    for m in re.finditer(r'pub const (\w+): &str = "\\u\{([0-9A-Fa-f]+)\}";', open(CODIGOS).read())
}

_fuentes = {}


def fuente(ruta):
    if ruta not in _fuentes:
        _fuentes[ruta] = TTFont(ruta)
    return _fuentes[ruta]


# ----------------------------------------------------------------------
#  Colores (como las carpetas de Solar: frente claro, fondo oscuro)
# ----------------------------------------------------------------------

def hex_rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


def rgb_hex(c):
    return "#%02X%02X%02X" % tuple(max(0, min(255, round(x))) for x in c)


def mezclar(a, b, t):
    a, b = hex_rgb(a), hex_rgb(b)
    return rgb_hex(tuple(a[i] + (b[i] - a[i]) * t for i in range(3)))


def paleta(base):
    if base.upper() == "#F97316":  # el naranja del logo, exacto
        return dict(frente=("#FDBA74", "#FB923C"), fondo=("#EA580C", "#C2410C"), emblema="#C2410C")
    return dict(
        frente=(mezclar(base, "#FFFFFF", 0.45), mezclar(base, "#FFFFFF", 0.18)),
        fondo=(mezclar(base, "#000000", 0.08), mezclar(base, "#000000", 0.28)),
        emblema=mezclar(base, "#000000", 0.38),
    )


# ----------------------------------------------------------------------
#  Trazos de glifos y texto
# ----------------------------------------------------------------------

def glifo(nombre, cx, cy, tam, linea=False):
    """Ícono Phosphor centrado en (cx, cy), del alto/ancho tam (relleno o de línea)."""
    f = fuente(PHOSPHOR_LINEA if linea else PHOSPHOR)
    g = f.getBestCmap()[CODIGO[nombre]]
    gs = f.getGlyphSet()
    b = BoundsPen(gs)
    gs[g].draw(b)
    x0, y0, x1, y1 = b.bounds
    esc = tam / max(x1 - x0, y1 - y0)
    pen = SVGPathPen(gs)
    # y hacia abajo en SVG
    t = TransformPen(pen, (esc, 0, 0, -esc, cx - (x0 + x1) / 2 * esc, cy + (y0 + y1) / 2 * esc))
    gs[g].draw(t)
    return pen.getCommands()


def medir(texto, ruta, tam):
    f = fuente(ruta)
    cmap, hmtx = f.getBestCmap(), f["hmtx"]
    upm = f["head"].unitsPerEm
    return sum(hmtx[cmap.get(ord(c), cmap[ord("?")])][0] for c in texto) * tam / upm


def texto(cad, x, y, ruta, tam, anclaje="inicio"):
    """Texto convertido a trazo; (x, y) es la línea base."""
    f = fuente(ruta)
    cmap, hmtx, gs = f.getBestCmap(), f["hmtx"], f.getGlyphSet()
    upm = f["head"].unitsPerEm
    esc = tam / upm
    ancho = medir(cad, ruta, tam)
    if anclaje == "centro":
        x -= ancho / 2
    elif anclaje == "fin":
        x -= ancho
    pen = SVGPathPen(gs)
    for c in cad:
        g = cmap.get(ord(c), cmap[ord("?")])
        gs[g].draw(TransformPen(pen, (esc, 0, 0, -esc, x, y)))
        x += hmtx[g][0] * esc
    return pen.getCommands()


# ----------------------------------------------------------------------
#  Carpeta con la forma del logo
# ----------------------------------------------------------------------

TRASERA = "M88 120 H284 L316 88 H424 Q448 88 448 112 V400 Q448 424 424 424 H88 Q64 424 64 400 V144 Q64 120 88 120 Z"
FRONTAL = "M144 176 H424 Q448 176 448 200 V400 Q448 424 424 424 H90 Q62 424 68 398 L120 194 Q125 176 144 176 Z"


def carpeta_grupo(id_, base, emblema=None, sol=False):
    p = paleta(base)
    defs = f"""<linearGradient id="{id_}t" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{p['fondo'][0]}"/><stop offset="1" stop-color="{p['fondo'][1]}"/></linearGradient>
<linearGradient id="{id_}f" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{p['frente'][0]}"/><stop offset="1" stop-color="{p['frente'][1]}"/></linearGradient>"""
    cuerpo = f'<path fill="url(#{id_}t)" d="{TRASERA}"/><path fill="url(#{id_}f)" d="{FRONTAL}"/>'
    if sol:
        rayos = "".join(
            f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}"/>'
            for x1, y1, x2, y2 in [
                (270, 242, 270, 218), (312.4, 259.6, 329.4, 242.6), (330, 302, 354, 302), (312.4, 344.4, 329.4, 361.4),
                (270, 362, 270, 386), (227.6, 344.4, 210.6, 361.4), (210, 302, 186, 302), (227.6, 259.6, 210.6, 242.6)]
        )
        cuerpo += (f'<g transform="translate(270 302) skewX(-14) translate(-270 -302)">'
                   f'<g stroke="{p["emblema"]}" stroke-width="14" stroke-linecap="round">{rayos}</g>'
                   f'<circle cx="270" cy="302" r="40" fill="{p["emblema"]}"/></g>')
    elif emblema:
        cuerpo += f'<path fill="{p["emblema"]}" d="{glifo(emblema, 272, 302, 128)}"/>'
    return defs, cuerpo


def carpeta_svg(nombre, base, emblema=None, sol=False):
    defs, cuerpo = carpeta_grupo("c", base, emblema, sol)
    svg = (f'<svg xmlns="http://www.w3.org/2000/svg" width="96" height="96" viewBox="48 48 416 416">'
           f'<defs>{defs}</defs>{cuerpo}</svg>\n')
    escribir(f"carpetas/{nombre}.svg", svg)


def archivo_svg(nombre, color, etiqueta):
    """Hoja con la esquina doblada y la extensión, como en la vista de Solar."""
    oscuro = mezclar(color, "#000000", 0.25)
    claro = mezclar(color, "#FFFFFF", 0.35)
    tam = 30 if len(etiqueta) <= 3 else 24
    letras = texto(etiqueta, 48, 74, INTER_NEGRITA, tam, "centro")
    svg = f"""<svg xmlns="http://www.w3.org/2000/svg" width="96" height="96" viewBox="0 0 96 96">
<defs><linearGradient id="h" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{claro}"/><stop offset="1" stop-color="{color}"/></linearGradient></defs>
<path fill="url(#h)" d="M22 6 H58 L80 28 V84 Q80 90 74 90 H22 Q16 90 16 84 V12 Q16 6 22 6 Z"/>
<path fill="{oscuro}" opacity=".55" d="M58 6 V22 Q58 28 64 28 H80 Z"/>
<path fill="#FFFFFF" d="{letras}"/>
</svg>
"""
    escribir(f"archivos/{nombre}.svg", svg)


def escribir(rel, contenido):
    ruta = os.path.join(AQUI, rel)
    os.makedirs(os.path.dirname(ruta), exist_ok=True)
    with open(ruta, "w", encoding="utf-8") as f:
        f.write(contenido)
    print("  ", rel)


# ----------------------------------------------------------------------
#  Las carpetas del README (una por sección)
# ----------------------------------------------------------------------

SECCIONES = [
    # archivo, color, emblema Phosphor
    ("que-trae", "#EAB308", "STAR"),
    ("capturas", "#A855F7", "CAMERA"),
    ("instalar", "#22C55E", "DOWNLOAD_SIMPLE"),
    ("uso", "#3B82F6", "KEYBOARD"),
    ("configuracion", "#64748B", "SLIDERS_HORIZONTAL"),
    ("temas", "#EC4899", "PALETTE"),
    ("actualizar", "#F97316", "ARROWS_CLOCKWISE"),
    ("desinstalar", "#EF4444", "TRASH"),
    ("como-esta-hecho", "#06B6D4", "CPU"),
    ("pruebas", "#14B8A6", "SHIELD_CHECK"),
    ("licencia", "#6366F1", "SCALES"),
    ("creditos", "#F43F5E", "HEART"),
]

ARCHIVOS = [
    ("rust", "#F97316", "RS"),
    ("c", "#3B82F6", "C"),
    ("lua", "#6366F1", "LUA"),
    ("toml", "#64748B", "TOML"),
    ("md", "#0EA5E9", "MD"),
    ("make", "#22C55E", "MAKE"),
    ("texto", "#94A3B8", "TXT"),
]


# ----------------------------------------------------------------------
#  Portada: una ventana de Solar abierta en la carpeta del proyecto
# ----------------------------------------------------------------------

def portada():
    W, H = 1200, 470
    fondo, superficie, borde = "#141110", "#1E1916", "#2A2420"
    tenue, claro, acento = "#A8A29E", "#F5F0EB", "#F97316"
    partes = []
    defs = ['<linearGradient id="brillo" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#F97316" stop-opacity=".22"/><stop offset=".55" stop-color="#F97316" stop-opacity="0"/></linearGradient>']

    # Ventana
    partes.append(f'<rect x="1" y="1" width="{W-2}" height="{H-2}" rx="18" fill="{fondo}" stroke="{borde}" stroke-width="2"/>')
    partes.append(f'<rect x="1" y="1" width="{W-2}" height="{H-2}" rx="18" fill="url(#brillo)"/>')
    # Barra de título
    partes.append(f'<path d="M19 1 H{W-19} Q{W-1} 1 {W-1} 19 V58 H1 V19 Q1 1 19 1 Z" fill="{superficie}"/>')
    partes.append(f'<line x1="1" y1="58" x2="{W-1}" y2="58" stroke="{borde}" stroke-width="2"/>')
    flechas = [("ARROW_LEFT", 30, claro), ("ARROW_RIGHT", 64, "#57534E"), ("ARROW_UP", 98, claro)]
    for g, x, c in flechas:
        partes.append(f'<path fill="{c}" d="{glifo(g, x, 30, 17, True)}"/>')
    # Ruta (migas)
    partes.append(f'<rect x="122" y="14" width="640" height="32" rx="9" fill="#2A231F"/>')
    partes.append(f'<path fill="{tenue}" d="{glifo("HOUSE", 140, 30, 15, True)}"/>')
    x = 156
    migas = ["Carpeta personal", "Proyectos", "Solar"]
    for i, m in enumerate(migas):
        ruta = INTER_MEDIA if i == len(migas) - 1 else INTER
        color = claro if i == len(migas) - 1 else tenue
        partes.append(f'<path fill="{color}" d="{texto(m, x, 35, ruta, 15)}"/>')
        x += medir(m, ruta, 15)
        if i < len(migas) - 1:
            partes.append(f'<path fill="#78716C" d="{glifo("CARET_RIGHT", x + 11, 30, 10, True)}"/>')
            x += 22
    # Botones de la derecha
    for g, xb in [("SIDEBAR_SIMPLE", 806), ("MAGNIFYING_GLASS", 844), ("EYE_SLASH", 882), ("LIST", 920)]:
        partes.append(f'<path fill="{tenue}" d="{glifo(g, xb, 30, 18, True)}"/>')
    partes.append(f'<rect x="942" y="13" width="34" height="34" rx="9" fill="#4A2A14"/>')
    partes.append(f'<path fill="{acento}" d="{glifo("GEAR", 959, 30, 18, True)}"/>')
    for g, xb in [("MINUS", 1088), ("CORNERS_OUT", 1124), ("X", 1160)]:
        partes.append(f'<path fill="{tenue}" d="{glifo(g, xb, 30, 16, True)}"/>')

    # Barra lateral
    partes.append(f'<path d="M1 59 H230 V{H-1} H19 Q1 {H-1} 1 {H-19} Z" fill="#18130F"/>')
    partes.append(f'<line x1="230" y1="59" x2="230" y2="{H-1}" stroke="{borde}" stroke-width="2"/>')
    partes.append(f'<path fill="{tenue}" d="{texto("LUGARES", 22, 92, INTER_MEDIA, 11.5)}"/>')
    lugares = [("HOUSE", "Carpeta personal", acento, False), ("FOLDER_SIMPLE", "Proyectos", "#3B82F6", True),
               ("GAME_CONTROLLER", "Juegos", "#22C55E", False), ("MUSIC_NOTES", "Música", acento, False),
               ("DOWNLOAD_SIMPLE", "Descargas", acento, False), ("TRASH", "Papelera", tenue, False)]
    y = 118
    for g, nombre, c, activo in lugares:
        if activo:
            partes.append(f'<rect x="12" y="{y-17}" width="206" height="32" rx="8" fill="#4A2A14"/>')
        partes.append(f'<path fill="{c}" d="{glifo(g, 32, y-1, 16, True)}"/>')
        partes.append(f'<path fill="{claro if activo else "#D6D3D1"}" d="{texto(nombre, 52, y+4, INTER_MEDIA if activo else INTER, 14)}"/>')
        y += 36
    partes.append(f'<path fill="{tenue}" d="{texto("DISPOSITIVOS", 22, y+14, INTER_MEDIA, 11.5)}"/>')
    y += 40
    partes.append(f'<path fill="{acento}" d="{glifo("USB", 32, y-1, 16, True)}"/>')
    partes.append(f'<path fill="#D6D3D1" d="{texto("Memoria USB", 52, y+4, INTER, 14)}"/>')
    partes.append(f'<rect x="52" y="{y+13}" width="150" height="4" rx="2" fill="#3A322C"/>')
    partes.append(f'<rect x="52" y="{y+13}" width="58" height="4" rx="2" fill="{acento}"/>')

    # Contenido: logo + título
    defs_logo, cuerpo_logo = carpeta_grupo("logo", "#F97316", sol=True)
    defs.append(defs_logo)
    partes.append(f'<g transform="translate(262 92) scale(0.43)">{cuerpo_logo}</g>')
    partes.append(f'<path fill="{claro}" d="{texto("Solar", 470, 190, INTER_NEGRITA, 78)}"/>')
    partes.append(f'<path fill="{tenue}" d="{texto("Gestor de archivos para Linux", 474, 232, INTER, 24)}"/>')
    x = 474
    for chip, color in [("Núcleo en C", "#3B82F6"), ("Interfaz en Rust", "#F97316"), ("Config en Lua", "#6366F1"),
                        ("v0.1.1-alpha", "#A8A29E"), ("GPL-3.0", "#22C55E")]:
        w = medir(chip, INTER_MEDIA, 13.5) + 24
        partes.append(f'<rect x="{x}" y="252" width="{w:.1f}" height="28" rx="14" fill="{color}" fill-opacity=".16" stroke="{color}" stroke-opacity=".45"/>')
        partes.append(f'<path fill="{color}" d="{texto(chip, x + 12, 271, INTER_MEDIA, 13.5)}"/>')
        x += w + 8

    # Fila de carpetas, como la cuadrícula de Solar
    fila = [("Juegos", "#22C55E", "GAME_CONTROLLER"), ("Proyectos", "#3B82F6", "CODE"), ("Fotos", "#A855F7", "CAMERA"),
            ("Música", "#F97316", "MUSIC_NOTES"), ("Escuela", "#EAB308", "GRADUATION_CAP"), ("Electrónica", "#374151", "CPU"),
            ("Capyro", "#EC4899", "HEADPHONES")]
    x0, paso = 262, 132
    for i, (nombre, color, em) in enumerate(fila):
        d, c = carpeta_grupo(f"f{i}", color, em)
        defs.append(d)
        x = x0 + i * paso
        if i == 0:  # seleccionada
            partes.append(f'<rect x="{x-6}" y="314" width="112" height="132" rx="12" fill="#4A2A14" stroke="{acento}" stroke-opacity=".7"/>')
        partes.append(f'<g transform="translate({x + 4 - 64*0.24:.1f} {326 - 88*0.24:.1f}) scale(0.24)">{c}</g>')
        partes.append(f'<path fill="{claro if i == 0 else "#D6D3D1"}" d="{texto(nombre, x + 50, 432, INTER_MEDIA if i == 0 else INTER, 13.5, "centro")}"/>')

    svg = (f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">'
           f'<defs>{"".join(defs)}</defs>{"".join(partes)}</svg>\n')
    escribir("portada.svg", svg)


def separador():
    """Barra de estado delgada que separa secciones."""
    W = 1200
    svg = f"""<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="6" viewBox="0 0 {W} 6">
<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#F97316" stop-opacity="0"/><stop offset=".5" stop-color="#F97316"/><stop offset="1" stop-color="#F97316" stop-opacity="0"/></linearGradient></defs>
<rect width="{W}" height="2" y="2" rx="1" fill="url(#g)"/></svg>
"""
    escribir("separador.svg", svg)


if __name__ == "__main__":
    print("Generando imágenes del README…")
    for nombre, color, emblema in SECCIONES:
        carpeta_svg(nombre, color, emblema)
    carpeta_svg("solar", "#F97316", sol=True)
    carpeta_svg("carpeta", "#F97316")
    for nombre, color, etiqueta in ARCHIVOS:
        archivo_svg(nombre, color, etiqueta)
    portada()
    separador()
    print("Listo.")
