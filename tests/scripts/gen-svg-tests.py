#!/usr/bin/env python3
import argparse
import re
import shutil

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SVG_DIR = ROOT / "assets" / "svgs"
SVG_FONT_DIR = ROOT / "assets" / "svg_fonts"
SVG_REF_DIR = ROOT / "refs" / "visreg" / "svg"
OUT_PATH = ROOT / "tests" / "src" / "svg_generated.rs"
DEFAULT_RESVG_DIR = ROOT.parent / "resvg"
DEFAULT_RESVG_TESTS_DIR = DEFAULT_RESVG_DIR / "crates" / "resvg" / "tests" / "tests"
DEFAULT_RESVG_FONTS_DIR = DEFAULT_RESVG_DIR / "crates" / "resvg" / "tests" / "fonts"
TEXT_FONT_FILE_SUFFIXES = {".md", ".txt"}

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
    "resvg_filters_feImage_svg.svg": NO_RELATIVE_PATHS,
    "resvg_masking_clipPath_image_is_not_a_valid_child.svg": NO_RELATIVE_PATHS,
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
    "resvg_structure_image_nested_external_png.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_raster_image_and_size_with_odd_numbers.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_recursive_1.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_recursive_2.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_width_and_height_set_to_auto.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_with_zero_width_and_height.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_zero_height.svg": NO_RELATIVE_PATHS,
    "resvg_structure_image_zero_width.svg": NO_RELATIVE_PATHS,
    "resvg_structure_style_external_CSS.svg": NO_RELATIVE_PATHS,
    "resvg_structure_use_xlink_to_an_external_file.svg": NO_RELATIVE_PATHS,
    "resvg_text_tref_link_to_an_external_file_element.svg": NO_RELATIVE_PATHS,

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

MANUAL_TESTS = {
    "custom_typst_issue_5509_1.svg",
    "custom_typst_issue_5509_2.svg",
    "custom_typst_issue_5509_3.svg",
    "issue199.svg",
    "issue291.svg",
    "issue293.svg",
    "small_text_with_filter.svg",
}

def upstream_test_name(path):
    stem = "_".join(path.with_suffix("").parts)
    stem = re.sub(r"[^A-Za-z0-9_]", "_", stem)
    return f"resvg_{stem}.svg"


def read_font_file(path):
    data = path.read_bytes()
    if path.suffix.lower() in TEXT_FONT_FILE_SUFFIXES:
        data = data.replace(b"\r\n", b"\n")
    return data


def ignored_test_reason(name):
    return IGNORE_TESTS.get(name)


def sync_resvg_tests(resvg_tests_dir):
    if not resvg_tests_dir.exists():
        raise FileNotFoundError(
            f"resvg test directory not found: {resvg_tests_dir}. "
            "Pass --resvg-tests-dir if resvg is not checked out next to krilla."
        )

    SVG_DIR.mkdir(parents=True, exist_ok=True)
    SVG_REF_DIR.mkdir(parents=True, exist_ok=True)

    copied_svgs = 0
    updated_svgs = 0
    copied_refs = 0

    for source_path in sorted(resvg_tests_dir.rglob("*.svg")):
        relative_path = source_path.relative_to(resvg_tests_dir)
        svg_name = upstream_test_name(relative_path)
        target_path = SVG_DIR / svg_name

        if not target_path.exists():
            copied_svgs += 1
        elif source_path.read_bytes() != target_path.read_bytes():
            updated_svgs += 1
        else:
            target_path = None

        if target_path is not None:
            shutil.copyfile(source_path, target_path)

        source_ref_path = source_path.with_suffix(".png")
        if not source_ref_path.exists():
            continue

        if ignored_test_reason(svg_name) is not None or svg_name in ADDITIONAL_ATTRS:
            continue

        target_ref_path = SVG_REF_DIR / svg_name.replace(".svg", ".png")

        if target_ref_path.exists():
            continue

        copied_refs += 1
        shutil.copyfile(source_ref_path, target_ref_path)

    return copied_svgs, updated_svgs, copied_refs


def sync_resvg_fonts(resvg_fonts_dir):
    if not resvg_fonts_dir.exists():
        raise FileNotFoundError(
            f"resvg font directory not found: {resvg_fonts_dir}. "
            "Pass --resvg-fonts-dir if resvg is not checked out next to krilla."
        )

    SVG_FONT_DIR.mkdir(parents=True, exist_ok=True)

    copied = 0
    updated = 0

    for source_path in sorted(p for p in resvg_fonts_dir.iterdir() if p.is_file()):
        target_path = SVG_FONT_DIR / source_path.name
        source_data = read_font_file(source_path)

        if not target_path.exists():
            copied += 1
        elif source_data != read_font_file(target_path):
            updated += 1
        else:
            continue

        target_path.write_bytes(source_data)

    return copied, updated


def write_generated_tests():
    test_string = f"// This file was auto-generated by `{Path(__file__).name}`, do not edit manually.\n\n"

    for p in sorted(SVG_DIR.glob("*.svg")):
        if p.name in MANUAL_TESTS:
            continue

        attrs = ["svg"]

        ignore_reason = ignored_test_reason(p.name)
        if ignore_reason is not None:
            test_string += f"// {ignore_reason}\n"
            attrs.append("ignore")

        if str(p.name) in ADDITIONAL_ATTRS:
            attrs.extend(ADDITIONAL_ATTRS[str(p.name)])
        
        test_string += f"#[visreg({', '.join(attrs)})] "
        
        test_string += f'fn {p.stem}() {{}}\n'

    with open(Path(OUT_PATH), "w") as file:
        file.write(test_string)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--resvg-tests-dir",
        type=Path,
        default=DEFAULT_RESVG_TESTS_DIR,
        help="Path to resvg/crates/resvg/tests/tests.",
    )
    parser.add_argument(
        "--resvg-fonts-dir",
        type=Path,
        default=DEFAULT_RESVG_FONTS_DIR,
        help="Path to resvg/crates/resvg/tests/fonts.",
    )
    parser.add_argument(
        "--no-sync",
        action="store_true",
        help="Only regenerate svg_generated.rs from the current assets/svgs directory.",
    )
    args = parser.parse_args()

    if not args.no_sync:
        copied_svgs, updated_svgs, copied_refs = sync_resvg_tests(
            args.resvg_tests_dir
        )
        copied_fonts, updated_fonts = sync_resvg_fonts(args.resvg_fonts_dir)
        print(f"synced resvg SVGs: {copied_svgs} new, {updated_svgs} updated")
        print(f"synced resvg references: {copied_refs} new")
        print(f"synced resvg fonts: {copied_fonts} new, {updated_fonts} updated")

    write_generated_tests()


if __name__ == "__main__":
    main()
