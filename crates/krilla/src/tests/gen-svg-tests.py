#!/usr/bin/env python3
import argparse
import shutil

from pathlib import Path

ROOT = Path(__file__).parent.parent.parent
SVG_DIR = ROOT / "assets" / "svgs"
OUT_PATH = ROOT / "src" / "tests" / "svg.rs"

NO_RELATIVE_PATHS = "no relative paths supported"
INVESTIGATE = "need to investigate"
NO_REFLECT = "spreadMethod reflect not supported"
NO_REPEAT = "spreadMethod repeat not supported"
NO_SUPPORT = "not supported in PDF"
NO_FONT = "font is not part of test suite yet"

IGNORE_TESTS = {
    # The following test cases still need to be investigated
    "resvg_painting_stroke_dasharray_n_0.svg": INVESTIGATE,
    "resvg_text_text_compound_emojis.svg": INVESTIGATE,
    "resvg_text_text_compound_emojis_and_coordinates_list.svg": INVESTIGATE,
    "resvg_text_text_emojis.svg": INVESTIGATE,

    # The following test cases need to be excluded due to technical reasons
    # and are not considered as wrong.
    "resvg_filters_feMorphology_huge_radius.svg": "will timeout CI",
    "resvg_filters_filter_huge_region.svg": "will sigkill",
    "resvg_structure_svg_negative_size.svg": "invalid size",
    "resvg_structure_svg_no_size.svg": "invalid size",
    "resvg_structure_svg_zero_size.svg": "invalid size",
    "resvg_structure_svg_not_UTF_8_encoding.svg": "invalid encoding",
    "resvg_filters_feImage_simple_case.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_image_with_float_size_scaling.svg": "too small in size",
    "resvg_painting_marker_with_an_image_child.svg": NO_RELATIVE_PATHS,
    "resvg_painting_mix_blend_mode_color_dodge.svg": "pdfium bug",
    "resvg_painting_stroke_linejoin_miter_clip.svg": NO_SUPPORT,
    "resvg_structure_image_external_gif.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_external_jpeg.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_external_png.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_external_webp.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_external_svg.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_external_svg_with_transform.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_external_svgz.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_float_size.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_no_height.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_no_height_on_svg.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_no_width.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_no_width_on_svg.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_no_width_and_height.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_no_width_and_height_on_svg.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_raster_image_and_size_with_odd_numbers.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_recursive_1.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_recursive_2.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_width_and_height_set_to_auto.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_zero_height.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_zero_width.svg": NO_RELATIVE_PATHS,

    # The following test cases should work but are not implemented in svg2pdf yet.
    "resvg_paint_servers_linearGradient_attributes_via_xlink_href_complex_order.svg": NO_REFLECT,
    "resvg_paint_servers_linearGradient_attributes_via_xlink_href_from_radialGradient.svg": NO_REFLECT,
    "resvg_paint_servers_linearGradient_spreadMethod_reflect.svg": NO_REFLECT,
    "resvg_paint_servers_linearGradient_spreadMethod_repeat.svg": NO_REPEAT,
    "resvg_paint_servers_radialGradient_attributes_via_xlink_href_complex_order.svg":NO_REFLECT,
    "resvg_paint_servers_radialGradient_attributes_via_xlink_href_from_linearGradient.svg": NO_REFLECT,
    "resvg_paint_servers_radialGradient_spreadMethod_reflect.svg": NO_REFLECT,
    "resvg_paint_servers_radialGradient_spreadMethod_repeat.svg": NO_REPEAT,
    "resvg_painting_stroke_linecap_zero_length_path_with_round.svg": "need to check how Chrome does it",
    "resvg_painting_stroke_linecap_zero_length_path_with_square.svg": "need to check how Firefox does it",
}

ADDITIONAL_ATTRS = {
    "resvg_masking_clip_rule_clip_rule_evenodd.svg": ["all"],
    "resvg_masking_mask_simple_case.svg": ["all"],
    "resvg_paint_servers_linearGradient_many_stops.svg": ["all"],
    "resvg_paint_servers_pattern_pattern_on_child.svg": ["all"],
    "resvg_paint_servers_radialGradient_many_stops.svg": ["all"],
    "resvg_painting_mix_blend_mode_exclusion.svg": ["all"],
    "resvg_text_textPath_closed_path.svg": ["all"]
}


def main():
    test_string = f"// This file was auto-generated by `{Path(__file__).name}`, do not edit manually.\n\n"
    test_string += "#![allow(non_snake_case)]\n\n"

    test_string += """
use krilla_macros::visreg;\n\n
"""

    for p in SVG_DIR.iterdir():
        attrs = ["svg"]

        if str(p.name) in IGNORE_TESTS:
            test_string += f"// {IGNORE_TESTS[str(p.name)]}\n"
            attrs.append("ignore")

        if str(p.name) in ADDITIONAL_ATTRS:
            attrs.extend(ADDITIONAL_ATTRS[str(p.name)])
        
        test_string += f"#[visreg({', '.join(attrs)})] "
        
        test_string += f'fn {p.stem}() {{}}\n'

    with open(Path(OUT_PATH), "w") as file:
        file.write(test_string)


if __name__ == "__main__":
    main()