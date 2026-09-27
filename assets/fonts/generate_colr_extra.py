#!/usr/bin/env python3
"""Generate COLR_extra.ttf using fontTools."""

import argparse
import struct
from pathlib import Path

from fontTools.colorLib.builder import buildCOLR
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont
from fontTools.ttLib.tables.DefaultTable import DefaultTable
from fontTools.ttLib.tables.otTables import CompositeMode, PaintFormat


DEFAULT_OUTPUT = Path(__file__).with_name("COLR_extra.ttf")
UNITS_PER_EM = 1000
TIMESTAMP = 3850070400

# RGBA
PALETTE = [(0, 0, 255, 255), (0, 255, 0, 255)]

OUTLINES = {
    "left": (0, 0, 250, 700),
    "right": (250, 0, 500, 700),
}


def solid(palette_index):
    return {
        "Format": PaintFormat.PaintSolid,
        "PaletteIndex": palette_index,
        "Alpha": 1.0,
    }


def solid_glyph(glyph, palette_index):
    return {
        "Format": PaintFormat.PaintGlyph,
        "Glyph": glyph,
        "Paint": solid(palette_index),
    }


COLOR_GLYPHS = {
    # Tests a nonzero CPAL palette offset; expected blue and green halves.
    "palette_offset": {
        "Format": PaintFormat.PaintColrLayers,
        "Layers": [solid_glyph("left", 0), solid_glyph("right", 1)],
    },
    # Tests unrestricted visible/mask paints in bounded In composites; blue/green.
    "unbounded_composite": {
        "Format": PaintFormat.PaintColrLayers,
        "Layers": [
            {
                "Format": PaintFormat.PaintComposite,
                "CompositeMode": CompositeMode.SRC_IN,
                "SourcePaint": solid(0),
                "BackdropPaint": solid_glyph("left", 1),
            },
            {
                "Format": PaintFormat.PaintComposite,
                "CompositeMode": CompositeMode.DEST_IN,
                "SourcePaint": solid(0),
                "BackdropPaint": solid_glyph("right", 1),
            },
        ],
    },
}


def build_palette():
    # Write CPAL directly to preserve palette 0's nonzero record offset.
    count = len(PALETTE)
    records = [(255, 0, 0, 255)] * count + PALETTE
    data = struct.pack(
        ">HHHHIHH", 0, count, 2, len(records), 16, count, 0
    )
    data += bytes(channel for r, g, b, a in records for channel in (b, g, r, a))
    table = DefaultTable("CPAL")
    table.data = data
    return table


def build_font():
    order = [".notdef", *COLOR_GLYPHS, *OUTLINES]
    assert len(order) == len(set(order)), "Glyph names must be unique"
    builder = FontBuilder(UNITS_PER_EM, isTTF=True)
    builder.setupGlyphOrder(order)
    builder.setupCharacterMap(
        {0xE000 + index: name for index, name in enumerate(COLOR_GLYPHS)}
    )

    glyphs = {}
    for name in order:
        pen = TTGlyphPen(None)
        if name in OUTLINES:
            x_min, y_min, x_max, y_max = OUTLINES[name]
            pen.moveTo((x_min, y_min))
            pen.lineTo((x_max, y_min))
            pen.lineTo((x_max, y_max))
            pen.lineTo((x_min, y_max))
            pen.closePath()
        glyphs[name] = pen.glyph()
    builder.setupGlyf(glyphs)
    builder.setupHorizontalMetrics(
        {name: (600, OUTLINES.get(name, (0,))[0]) for name in order}
    )
    builder.setupHorizontalHeader(ascent=800, descent=-200)
    builder.setupNameTable(
        {
            "familyName": "COLR Extra",
            "styleName": "Regular",
            "uniqueFontIdentifier": "COLRExtra-Regular",
            "fullName": "COLR Extra Regular",
            "psName": "COLRExtra-Regular",
            "version": "Version 1.000",
        }
    )
    builder.setupOS2(
        sTypoAscender=800, sTypoDescender=-200,
        usWinAscent=800, usWinDescent=200,
    )
    builder.setupPost()
    builder.setupMaxp()
    font = builder.font
    font["COLR"] = buildCOLR(COLOR_GLYPHS, glyphMap=font.getReverseGlyphMap())
    font["CPAL"] = build_palette()
    font["head"].created = font["head"].modified = TIMESTAMP
    font.recalcTimestamp = False
    return font


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    build_font().save(args.output)

    with TTFont(args.output, recalcTimestamp=False) as font:
        data = font.getTableData("CPAL")
        assert struct.unpack_from(">H", data, 12)[0] == len(PALETTE)
        assert [tuple(c) for c in font["CPAL"].palettes[0]] == [
            (b, g, r, a) for r, g, b, a in PALETTE
        ]
        print(f"Generated {args.output} ({len(font.getGlyphOrder())} glyphs)")
        for name in COLOR_GLYPHS:
            print(f"  {font.getGlyphID(name)}: {name}")


if __name__ == "__main__":
    main()
