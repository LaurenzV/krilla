#!/usr/bin/env python3

import argparse
import struct
import zlib
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Optional


ROOT = Path(__file__).resolve().parents[3]
DEFAULT_LUMA_SOURCE = ROOT / "assets" / "images" / "luma8.png"
DEFAULT_RGB_SOURCE = ROOT / "assets" / "images" / "rgb8.png"
DEFAULT_OUTPUT_DIR = Path(__file__).resolve().parent
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
ADAM7_PASSES = (
    (0, 0, 8, 8),
    (4, 0, 8, 8),
    (0, 4, 4, 8),
    (2, 0, 4, 4),
    (0, 2, 2, 4),
    (1, 0, 2, 2),
    (0, 1, 1, 2),
)


@dataclass(frozen=True)
class SourceImage:
    width: int
    height: int
    channels: int
    pixels: tuple


@dataclass(frozen=True)
class OutputSpec:
    name: str
    color_type: int
    bit_depth: int
    filter_type: Optional[int] = None
    interlaced: bool = False
    transparency: bool = False
    binary_transparency: bool = False
    idat_chunks: int = 1
    icc_profile: Optional[str] = None


OUTPUT_SPECS = (
    OutputSpec("grayscale_1.png", 0, 1),
    OutputSpec("grayscale_2.png", 0, 2),
    OutputSpec("grayscale_4.png", 0, 4),
    OutputSpec("grayscale_8.png", 0, 8),
    OutputSpec("grayscale_16.png", 0, 16),
    OutputSpec("grayscale_8_icc.png", 0, 8, icc_profile="sGrey-v4.icc"),
    OutputSpec("grayscale_trns_1.png", 0, 1, transparency=True),
    OutputSpec("grayscale_trns_2.png", 0, 2, transparency=True),
    OutputSpec("grayscale_trns_4.png", 0, 4, transparency=True),
    OutputSpec("grayscale_trns_8.png", 0, 8, transparency=True),
    OutputSpec("grayscale_trns_16.png", 0, 16, transparency=True),
    OutputSpec("grayscale_8_interlaced.png", 0, 8, interlaced=True),
    OutputSpec("rgb_8.png", 2, 8),
    OutputSpec("rgb_16.png", 2, 16),
    OutputSpec("rgb_8_icc.png", 2, 8, icc_profile="sRGB-v4.icc"),
    OutputSpec("rgb_8_multiple_idat.png", 2, 8, idat_chunks=3),
    OutputSpec("rgb_trns_8.png", 2, 8, transparency=True),
    OutputSpec("rgb_trns_16.png", 2, 16, transparency=True),
    OutputSpec("rgb_indexed_1.png", 3, 1),
    OutputSpec("rgb_indexed_2.png", 3, 2),
    OutputSpec("rgb_indexed_4.png", 3, 4),
    OutputSpec("rgb_indexed_8.png", 3, 8),
    OutputSpec("rgb_indexed_trns_1.png", 3, 1, transparency=True),
    OutputSpec("rgb_indexed_trns_2.png", 3, 2, transparency=True),
    OutputSpec("rgb_indexed_trns_4.png", 3, 4, transparency=True),
    OutputSpec("rgb_indexed_trns_8.png", 3, 8, transparency=True),
    OutputSpec(
        "rgb_indexed_trns_binary_8.png",
        3,
        8,
        transparency=True,
        binary_transparency=True,
    ),
    OutputSpec("grayscale_alpha_8.png", 4, 8),
    OutputSpec("grayscale_alpha_16.png", 4, 16),
    OutputSpec("rgba_8.png", 6, 8),
    OutputSpec("rgba_16.png", 6, 16),
    OutputSpec("rgb_8_filter_none.png", 2, 8, 0),
    OutputSpec("rgb_8_filter_sub.png", 2, 8, 1),
    OutputSpec("rgb_8_filter_up.png", 2, 8, 2),
    OutputSpec("rgb_8_filter_average.png", 2, 8, 3),
    OutputSpec("rgb_8_filter_paeth.png", 2, 8, 4),
)


def paeth(left, above, upper_left):
    estimate = left + above - upper_left
    left_distance = abs(estimate - left)
    above_distance = abs(estimate - above)
    upper_left_distance = abs(estimate - upper_left)
    if left_distance <= above_distance and left_distance <= upper_left_distance:
        return left
    if above_distance <= upper_left_distance:
        return above
    return upper_left


def unfilter_row(filter_type, filtered, previous, bytes_per_pixel):
    result = bytearray(len(filtered))
    for index, value in enumerate(filtered):
        left = result[index - bytes_per_pixel] if index >= bytes_per_pixel else 0
        above = previous[index] if previous else 0
        upper_left = (
            previous[index - bytes_per_pixel]
            if previous and index >= bytes_per_pixel
            else 0
        )
        if filter_type == 0:
            predictor = 0
        elif filter_type == 1:
            predictor = left
        elif filter_type == 2:
            predictor = above
        elif filter_type == 3:
            predictor = (left + above) // 2
        elif filter_type == 4:
            predictor = paeth(left, above, upper_left)
        else:
            raise ValueError(f"unsupported PNG filter type {filter_type}")
        result[index] = (value + predictor) & 0xFF
    return bytes(result)


def read_chunks(data):
    if not data.startswith(PNG_SIGNATURE):
        raise ValueError("invalid PNG signature")

    offset = len(PNG_SIGNATURE)
    while offset < len(data):
        if len(data) - offset < 12:
            raise ValueError("truncated PNG chunk")
        length = struct.unpack_from(">I", data, offset)[0]
        chunk_end = offset + 12 + length
        if chunk_end > len(data):
            raise ValueError("truncated PNG chunk")
        chunk_type = data[offset + 4 : offset + 8]
        payload = data[offset + 8 : offset + 8 + length]
        expected_crc = struct.unpack_from(">I", data, offset + 8 + length)[0]
        actual_crc = zlib.crc32(chunk_type + payload) & 0xFFFFFFFF
        if actual_crc != expected_crc:
            raise ValueError(f"CRC mismatch in {chunk_type.decode('ascii')}")
        yield chunk_type, payload
        offset = chunk_end
        if chunk_type == b"IEND":
            break


def decode_source(path, expected_color_type):
    chunks = list(read_chunks(path.read_bytes()))
    ihdr_chunks = [payload for chunk_type, payload in chunks if chunk_type == b"IHDR"]
    if len(ihdr_chunks) != 1 or len(ihdr_chunks[0]) != 13:
        raise ValueError(f"{path} does not contain exactly one valid IHDR chunk")

    width, height, bit_depth, color_type, compression, filtering, interlace = struct.unpack(
        ">IIBBBBB", ihdr_chunks[0]
    )
    if (bit_depth, color_type, compression, filtering, interlace) != (
        8,
        expected_color_type,
        0,
        0,
        0,
    ):
        raise ValueError(
            f"{path} must be a non-interlaced 8-bit PNG with color type "
            f"{expected_color_type}"
        )

    channels = 1 if color_type == 0 else 3
    row_size = width * channels
    compressed = b"".join(
        payload for chunk_type, payload in chunks if chunk_type == b"IDAT"
    )
    filtered = zlib.decompress(compressed)
    expected_size = height * (row_size + 1)
    if len(filtered) != expected_size:
        raise ValueError(
            f"{path} has {len(filtered)} decompressed bytes, expected {expected_size}"
        )

    rows = []
    previous = b""
    offset = 0
    for _ in range(height):
        filter_type = filtered[offset]
        offset += 1
        row = unfilter_row(
            filter_type,
            filtered[offset : offset + row_size],
            previous,
            channels,
        )
        rows.append(row)
        previous = row
        offset += row_size

    if channels == 1:
        pixels = tuple(value for row in rows for value in row)
    else:
        pixels = tuple(
            tuple(row[index : index + channels])
            for row in rows
            for index in range(0, len(row), channels)
        )
    return SourceImage(width, height, channels, pixels)


def linear_gradient_value(x, width):
    if width == 1:
        return 255
    return (x * 255 + (width - 1) // 2) // (width - 1)


def symmetric_gradient_value(x, width):
    if width <= 2:
        return 255
    half_width = (width - 1) // 2
    distance_from_edge = min(x, width - 1 - x)
    return min(255, (distance_from_edge * 255 + half_width // 2) // half_width)


def make_blue_red_gradient(width, height):
    luma_row = tuple(linear_gradient_value(x, width) for x in range(width))
    rgb_row = tuple((value, 0, 255 - value) for value in luma_row)
    return (
        SourceImage(width, height, 1, luma_row * height),
        SourceImage(width, height, 3, rgb_row * height),
    )


def quantize_sample(value, bit_depth):
    level_count = 1 << bit_depth
    return min(level_count - 1, value * level_count // 256)


def encode_samples(samples, bit_depth):
    if bit_depth == 8:
        return bytes(samples)
    if bit_depth == 16:
        return b"".join(struct.pack(">H", value) for value in samples)

    packed = bytearray()
    per_byte = 8 // bit_depth
    for start in range(0, len(samples), per_byte):
        value = 0
        for sample in samples[start : start + per_byte]:
            value = (value << bit_depth) | sample
        missing = per_byte - len(samples[start : start + per_byte])
        packed.append(value << (missing * bit_depth))
    return bytes(packed)


def filter_row(filter_type, row, previous, bytes_per_pixel):
    result = bytearray(len(row))
    for index, value in enumerate(row):
        left = row[index - bytes_per_pixel] if index >= bytes_per_pixel else 0
        above = previous[index] if previous else 0
        upper_left = (
            previous[index - bytes_per_pixel]
            if previous and index >= bytes_per_pixel
            else 0
        )
        if filter_type == 0:
            predictor = 0
        elif filter_type == 1:
            predictor = left
        elif filter_type == 2:
            predictor = above
        elif filter_type == 3:
            predictor = (left + above) // 2
        else:
            predictor = paeth(left, above, upper_left)
        result[index] = (value - predictor) & 0xFF
    return bytes(result)


def filter_rows(rows, bytes_per_pixel, forced_filter_type=None):
    output = bytearray()
    previous = b""
    for row in rows:
        if forced_filter_type is None:
            candidates = [
                filter_row(filter_type, row, previous, bytes_per_pixel)
                for filter_type in range(5)
            ]
            filter_type = min(
                range(5),
                key=lambda candidate: sum(
                    min(value, 256 - value) for value in candidates[candidate]
                ),
            )
            filtered = candidates[filter_type]
        else:
            filter_type = forced_filter_type
            filtered = filter_row(filter_type, row, previous, bytes_per_pixel)
        output.append(filter_type)
        output.extend(filtered)
        previous = row
    return bytes(output)


def split_color_box(colors):
    ranges = [
        max(color[channel] for color, _ in colors)
        - min(color[channel] for color, _ in colors)
        for channel in range(3)
    ]
    channel = max(range(3), key=lambda index: (ranges[index], -index))
    colors = sorted(colors, key=lambda item: (item[0][channel], item[0]))
    halfway = sum(count for _, count in colors) / 2
    accumulated = 0
    split_at = 1
    for split_at, (_, count) in enumerate(colors, start=1):
        accumulated += count
        if accumulated >= halfway:
            break
    split_at = min(max(split_at, 1), len(colors) - 1)
    return colors[:split_at], colors[split_at:]


def quantize_palette(pixels, maximum_colors):
    boxes = [list(Counter(pixels).items())]
    while len(boxes) < maximum_colors:
        candidates = [
            (index, box)
            for index, box in enumerate(boxes)
            if len(box) > 1
        ]
        if not candidates:
            break

        def split_priority(item):
            _, box = item
            ranges = [
                max(color[channel] for color, _ in box)
                - min(color[channel] for color, _ in box)
                for channel in range(3)
            ]
            population = sum(count for _, count in box)
            return max(ranges) * population, max(ranges), population, len(box)

        index, box = max(candidates, key=split_priority)
        first, second = split_color_box(box)
        boxes[index : index + 1] = [first, second]

    palette = []
    color_indices = {}
    for palette_index, box in enumerate(boxes):
        population = sum(count for _, count in box)
        palette.append(
            tuple(
                (sum(color[channel] * count for color, count in box) + population // 2)
                // population
                for channel in range(3)
            )
        )
        for color, _ in box:
            color_indices[color] = palette_index

    return palette, tuple(color_indices[pixel] for pixel in pixels)


def alpha_at(x, width, bit_depth):
    maximum = (1 << bit_depth) - 1
    minimum = (maximum + 1) // 2
    gradient = linear_gradient_value(x, width)
    return minimum + (gradient * (maximum - minimum) + 127) // 255


def inside_center_square(x, y, width, height, size=50):
    x_start = (width - size) // 2
    y_start = (height - size) // 2
    return x_start <= x < x_start + size and y_start <= y < y_start + size


def build_rows(spec, luma, rgb):
    source = luma if spec.color_type in (0, 4) else rgb
    rows = []
    palette = None
    transparency = None

    if spec.color_type == 3:
        palette, indices = quantize_palette(rgb.pixels, 1 << spec.bit_depth)
        for y in range(rgb.height):
            start = y * rgb.width
            rows.append(encode_samples(indices[start : start + rgb.width], spec.bit_depth))
        if spec.transparency:
            if spec.binary_transparency:
                transparent_index = len(palette) // 2
                transparency = bytes([255] * transparent_index + [0])
            else:
                transparency = bytes(alpha_at(color[0], 256, 8) for color in palette)
        return rows, palette, transparency, 1

    channels = {0: 1, 2: 3, 4: 2, 6: 4}[spec.color_type]
    for y in range(source.height):
        samples = []
        for x in range(source.width):
            pixel = source.pixels[y * source.width + x]
            values = (pixel,) if isinstance(pixel, int) else pixel
            if spec.transparency and inside_center_square(
                x, y, source.width, source.height
            ):
                values = (0,) * len(values)
            if spec.bit_depth == 16:
                values = tuple(value * 257 for value in values)
            elif spec.bit_depth < 8:
                values = tuple(quantize_sample(value, spec.bit_depth) for value in values)
            if spec.color_type in (4, 6):
                alpha = alpha_at(x, source.width, spec.bit_depth)
                values = (*values, alpha)
            samples.extend(values)
        rows.append(encode_samples(samples, spec.bit_depth))

    if spec.transparency:
        if spec.color_type == 0:
            transparency = struct.pack(">H", 0)
        elif spec.color_type == 2:
            transparency = struct.pack(">HHH", 0, 0, 0)

    bytes_per_pixel = max(1, (channels * spec.bit_depth + 7) // 8)
    return rows, palette, transparency, bytes_per_pixel


def make_chunk(chunk_type, payload):
    checksum = zlib.crc32(chunk_type + payload) & 0xFFFFFFFF
    return struct.pack(">I", len(payload)) + chunk_type + payload + struct.pack(">I", checksum)


def filter_adam7_grayscale8(rows, width, height, forced_filter_type=None):
    output = bytearray()
    for x_start, y_start, x_step, y_step in ADAM7_PASSES:
        pass_rows = [
            bytes(rows[y][x] for x in range(x_start, width, x_step))
            for y in range(y_start, height, y_step)
        ]
        output.extend(filter_rows(pass_rows, 1, forced_filter_type))
    return bytes(output)


def write_png(
    path, spec, width, height, rows, palette, transparency, bytes_per_pixel
):
    interlace = int(spec.interlaced)
    ihdr = struct.pack(
        ">IIBBBBB",
        width,
        height,
        spec.bit_depth,
        spec.color_type,
        0,
        0,
        interlace,
    )
    chunks = [make_chunk(b"IHDR", ihdr)]
    if spec.icc_profile is not None:
        profile = (ROOT / "crates" / "krilla" / "icc" / spec.icc_profile).read_bytes()
        chunks.append(
            make_chunk(b"iCCP", b"ICC Profile\0\0" + zlib.compress(profile, level=9))
        )
    if palette is not None:
        chunks.append(
            make_chunk(b"PLTE", bytes(channel for color in palette for channel in color))
        )
    if transparency is not None:
        chunks.append(make_chunk(b"tRNS", transparency))
    if spec.interlaced:
        if (spec.color_type, spec.bit_depth) != (0, 8):
            raise ValueError("Adam7 generation currently supports only 8-bit grayscale")
        filtered = filter_adam7_grayscale8(
            rows, width, height, spec.filter_type
        )
    else:
        filtered = filter_rows(rows, bytes_per_pixel, spec.filter_type)
    compressed = zlib.compress(filtered, level=9)
    for index in range(spec.idat_chunks):
        start = len(compressed) * index // spec.idat_chunks
        end = len(compressed) * (index + 1) // spec.idat_chunks
        chunks.append(make_chunk(b"IDAT", compressed[start:end]))
    chunks.append(make_chunk(b"IEND", b""))
    path.write_bytes(PNG_SIGNATURE + b"".join(chunks))


def verify_png(path, expected_spec, width, height):
    chunks = list(read_chunks(path.read_bytes()))
    ihdr = next(payload for chunk_type, payload in chunks if chunk_type == b"IHDR")
    actual = struct.unpack(">IIBBBBB", ihdr)
    expected = (
        width,
        height,
        expected_spec.bit_depth,
        expected_spec.color_type,
        0,
        0,
        int(expected_spec.interlaced),
    )
    if actual != expected:
        raise ValueError(f"{path} has IHDR {actual}, expected {expected}")
    transparency = [
        payload for chunk_type, payload in chunks if chunk_type == b"tRNS"
    ]
    if len(transparency) != int(expected_spec.transparency):
        raise ValueError(f"{path} has an unexpected number of tRNS chunks")
    if expected_spec.transparency:
        if expected_spec.color_type == 0 and transparency[0] != b"\0\0":
            raise ValueError(f"{path} does not mark black as transparent")
        if expected_spec.color_type == 2 and transparency[0] != b"\0\0\0\0\0\0":
            raise ValueError(f"{path} does not mark black as transparent")
        if expected_spec.color_type == 3:
            palette = next(
                payload for chunk_type, payload in chunks if chunk_type == b"PLTE"
            )
            if expected_spec.binary_transparency:
                if transparency[0][-1] != 0 or any(
                    alpha != 255 for alpha in transparency[0][:-1]
                ):
                    raise ValueError(f"{path} does not contain binary transparency")
            elif len(transparency[0]) != len(palette) // 3:
                raise ValueError(f"{path} does not define alpha for every palette entry")
    icc_profiles = [
        payload for chunk_type, payload in chunks if chunk_type == b"iCCP"
    ]
    if len(icc_profiles) != int(expected_spec.icc_profile is not None):
        raise ValueError(f"{path} has an unexpected number of iCCP chunks")
    if expected_spec.icc_profile is not None:
        name, encoded_profile = icc_profiles[0].split(b"\0", 1)
        if name != b"ICC Profile" or encoded_profile[0] != 0:
            raise ValueError(f"{path} has an invalid iCCP chunk")
        expected_profile = (
            ROOT / "crates" / "krilla" / "icc" / expected_spec.icc_profile
        ).read_bytes()
        if zlib.decompress(encoded_profile[1:]) != expected_profile:
            raise ValueError(f"{path} has an unexpected ICC profile")
    compressed = b"".join(
        payload for chunk_type, payload in chunks if chunk_type == b"IDAT"
    )
    idat_count = sum(chunk_type == b"IDAT" for chunk_type, _ in chunks)
    if idat_count != expected_spec.idat_chunks:
        raise ValueError(f"{path} has {idat_count} IDAT chunks")
    filtered = zlib.decompress(compressed)
    channels = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}[expected_spec.color_type]
    row_size = (width * channels * expected_spec.bit_depth + 7) // 8
    if expected_spec.interlaced:
        offset = 0
        filter_types = []
        for x_start, y_start, x_step, y_step in ADAM7_PASSES:
            pass_width = len(range(x_start, width, x_step))
            pass_height = len(range(y_start, height, y_step))
            pass_row_size = (
                pass_width * channels * expected_spec.bit_depth + 7
            ) // 8
            for _ in range(pass_height):
                if offset + pass_row_size + 1 > len(filtered):
                    raise ValueError(f"{path} has truncated Adam7 data")
                filter_types.append(filtered[offset])
                offset += pass_row_size + 1
        if offset != len(filtered):
            raise ValueError(f"{path} has an unexpected decompressed size")
    else:
        if len(filtered) != height * (row_size + 1):
            raise ValueError(f"{path} has an unexpected decompressed size")
        filter_types = [filtered[row * (row_size + 1)] for row in range(height)]
    if any(filter_type not in range(5) for filter_type in filter_types):
        raise ValueError(f"{path} contains an invalid filter type")
    if expected_spec.filter_type is not None and any(
        filter_type != expected_spec.filter_type for filter_type in filter_types
    ):
        raise ValueError(
            f"{path} does not use filter type {expected_spec.filter_type} for every row"
        )


def main():
    parser = argparse.ArgumentParser(
        description="Generate PNG assets covering color types, bit depths, and filters."
    )
    parser.add_argument("--luma-source", type=Path, default=DEFAULT_LUMA_SOURCE)
    parser.add_argument("--rgb-source", type=Path, default=DEFAULT_RGB_SOURCE)
    parser.add_argument("--output-dir", type=Path, default=DEFAULT_OUTPUT_DIR)
    parser.add_argument("--width", type=int, default=151)
    parser.add_argument("--height", type=int, default=151)
    parser.add_argument(
        "--use-source-images",
        action="store_true",
        help="Use --luma-source and --rgb-source instead of the blue-to-red gradient.",
    )
    args = parser.parse_args()

    if args.width <= 0 or args.height <= 0:
        parser.error("--width and --height must be positive")

    if args.use_source_images:
        luma = decode_source(args.luma_source, expected_color_type=0)
        rgb = decode_source(args.rgb_source, expected_color_type=2)
    else:
        luma, rgb = make_blue_red_gradient(args.width, args.height)
    if (luma.width, luma.height) != (rgb.width, rgb.height):
        raise ValueError("luma and RGB source dimensions must match")

    args.output_dir.mkdir(parents=True, exist_ok=True)
    for spec in OUTPUT_SPECS:
        rows, palette, transparency, bytes_per_pixel = build_rows(spec, luma, rgb)
        output_path = args.output_dir / spec.name
        write_png(
            output_path,
            spec,
            luma.width,
            luma.height,
            rows,
            palette,
            transparency,
            bytes_per_pixel,
        )
        verify_png(output_path, spec, luma.width, luma.height)
        print(
            f"generated {output_path.relative_to(ROOT)} "
            f"(color type {spec.color_type}, {spec.bit_depth}-bit)"
        )


if __name__ == "__main__":
    main()
