//! PostScript names for instances of variable fonts.
//!
//! The name ID 6 of a variable font names its default instance, so it can't be used
//! for other instances. Instead, we generate a name as described in Adobe Technical
//! Note #5902, the same way FreeType does.

use std::fmt::Write;

use skrifa::raw::types::{Fixed, NameId};
use skrifa::raw::TableProvider;
use skrifa::{FontRef, MetadataProvider};

use crate::text::cid::MAX_POSTSCRIPT_NAME_LEN;
use crate::text::Tag;
use crate::util::stable_hash128;

/// The length of a hyphen, a 128-bit hash in hexadecimal and three periods,
/// which end a last resort name.
const LAST_RESORT_LEN: usize = 1 + 32 + 3;

/// Generate the PostScript name of the instance of a variable font at the given
/// variation coordinates.
///
/// Returns `None` if the coordinates select the default instance, which is named by
/// name ID 6, or if the font names neither the instance nor its family in a way a
/// name can be derived from.
pub(crate) fn instance_postscript_name(
    font_ref: &FontRef<'_>,
    var_coords: &[(Tag, f32)],
) -> Option<String> {
    let fvar = font_ref.fvar().ok()?;
    let axes = fvar.axes().ok()?;

    // Resolve the coordinates the same way as `skrifa::Axes::location`: The last
    // setting for an axis wins and is clamped to the range of the axis.
    let coords = axes
        .iter()
        .map(|axis| {
            let tag = axis.axis_tag().to_be_bytes();
            var_coords
                .iter()
                .rev()
                .find(|(t, _)| *t.get() == tag)
                .map_or(axis.default_value(), |(_, value)| {
                    Fixed::from_f64(f64::from(*value))
                        .max(axis.min_value())
                        .min(axis.max_value())
                })
        })
        .collect::<Vec<_>>();

    if axes
        .iter()
        .zip(&coords)
        .all(|(axis, coord)| axis.default_value() == *coord)
    {
        return None;
    }

    let named_instance = fvar.instances().ok().and_then(|instances| {
        instances.iter().find_map(|instance| {
            let instance = instance.ok()?;
            let coordinates = instance.coordinates.iter().map(|c| c.get());
            coordinates.eq(coords.iter().copied()).then_some(instance)
        })
    });

    // A PostScript name that the font gives a named instance doesn't need a prefix.
    if let Some(name) = named_instance
        .as_ref()
        .and_then(|instance| instance.post_script_name_id)
        .and_then(|id| name_string(font_ref, id))
        .filter(|name| !name.is_empty())
    {
        return Some(shorten(&name, &name));
    }

    let prefix = [
        NameId::VARIATIONS_POSTSCRIPT_NAME_PREFIX,
        NameId::TYPOGRAPHIC_FAMILY_NAME,
        NameId::FAMILY_NAME,
    ]
    .into_iter()
    .find_map(|id| name_string(font_ref, id))
    .map(alphanumeric)
    .filter(|prefix| !prefix.is_empty())?;

    let subfamily = named_instance
        .and_then(|instance| name_string(font_ref, instance.subfamily_name_id))
        .map(alphanumeric)
        .filter(|subfamily| !subfamily.is_empty());

    let name = subfamily.map_or_else(
        || {
            let mut name = prefix.clone();
            for (axis, coord) in axes.iter().zip(&coords) {
                if axis.default_value() != *coord {
                    name.push('_');
                    write_fixed(&mut name, *coord);
                    name.extend(
                        axis.axis_tag()
                            .to_be_bytes()
                            .into_iter()
                            .filter(u8::is_ascii_alphanumeric)
                            .map(char::from),
                    );
                }
            }
            name
        },
        |subfamily| format!("{prefix}-{subfamily}"),
    );

    Some(shorten(&name, &prefix))
}

/// Return `name` if it is short enough to be written into a PDF.
///
/// Otherwise, return a last resort name: as much of `stem` as fits, a hyphen, a hash
/// of the full name and three periods. Distinct names thus stay distinct.
fn shorten(name: &str, stem: &str) -> String {
    if name.len() <= MAX_POSTSCRIPT_NAME_LEN {
        return name.to_string();
    }

    let mut end = stem.len().min(MAX_POSTSCRIPT_NAME_LEN - LAST_RESORT_LEN);
    while !stem.is_char_boundary(end) {
        end -= 1;
    }

    format!("{}-{:032X}...", &stem[..end], stable_hash128(name))
}

fn name_string(font_ref: &FontRef<'_>, id: NameId) -> Option<String> {
    font_ref
        .localized_strings(id)
        .english_or_first()
        .map(|string| string.chars().collect())
}

fn alphanumeric(string: String) -> String {
    string.chars().filter(char::is_ascii_alphanumeric).collect()
}

/// Write the shortest decimal representation of a 16.16 fixed-point number that
/// converts back to the same number, omitting leading and trailing zeros.
///
/// This is a port of `fixed2float` in FreeType's `sfdriver.c`.
fn write_fixed(name: &mut String, value: Fixed) {
    let mut fixed = i64::from(value.to_bits());

    if fixed == 0 {
        name.push('0');
        return;
    }

    if fixed < 0 {
        name.push('-');
        fixed = -fixed;
    }

    let int_part = fixed >> 16;
    let mut frac_part = fixed & 0xFFFF;

    if int_part > 0 {
        write!(name, "{int_part}").unwrap();
    }

    if frac_part == 0 {
        return;
    }

    // Get at most five digits of the fractional part, rounding the last one up.
    let mut digits = Vec::with_capacity(5);
    frac_part = frac_part * 10 + 5;
    for _ in 0..5 {
        digits.push(b'0' + (frac_part / 0x10000) as u8);
        frac_part %= 0x10000;
        if frac_part == 0 {
            break;
        }
        frac_part *= 10;
    }

    // With five digits, the value one lower in the last digit may be an equivalent
    // representation as well. If so, prefer the one ending in zero, and otherwise the
    // one with less error (ties to even). FreeType found these bounds by testing all
    // possible values.
    if digits.len() == 5 {
        let last = &mut digits[4];
        if frac_part < 34480 * 10 && *last == b'1' {
            *last = b'0';
        } else if (frac_part == 17232 * 10 && *last % 2 == 1)
            || (frac_part < 17232 * 10 && *last != b'0')
        {
            *last -= 1;
        }
    }

    while digits.last() == Some(&b'0') {
        digits.pop();
    }

    name.push('.');
    name.extend(digits.into_iter().map(char::from));
}

#[cfg(test)]
mod tests {
    use skrifa::raw::types::Fixed;
    use skrifa::FontRef;

    use super::{instance_postscript_name, write_fixed};
    use crate::text::cid::MAX_POSTSCRIPT_NAME_LEN;
    use crate::text::Tag;
    use crate::util::test_utils::{CANTARELL_VAR, NOTO_SANS_VAR};

    fn fixed(value: Fixed) -> String {
        let mut name = String::new();
        write_fixed(&mut name, value);
        name
    }

    fn name(data: &[u8], coords: &[(&[u8; 4], f32)]) -> Option<String> {
        let font_ref = FontRef::new(data).unwrap();
        let coords = coords
            .iter()
            .map(|(tag, value)| (Tag::new(tag), *value))
            .collect::<Vec<_>>();
        instance_postscript_name(&font_ref, &coords)
    }

    /// Build a font with only the `fvar` and `name` tables, which is all that naming an
    /// instance reads.
    ///
    /// - `names`: language ID, name ID and string of each name record.
    /// - `axes`: tag, minimum, default and maximum value of each axis.
    /// - `instances`: subfamily name ID, PostScript name ID and coordinates of each
    ///   named instance.
    fn test_font(
        names: &[(u16, u16, &str)],
        axes: &[(&[u8; 4], f64, f64, f64)],
        instances: &[(u16, Option<u16>, &[f64])],
    ) -> Vec<u8> {
        fn fixed(value: f64) -> [u8; 4] {
            Fixed::from_f64(value).to_bits().to_be_bytes()
        }

        let mut names = names.to_vec();
        names.sort_by_key(|&(language, id, _)| (language, id));
        let mut name = vec![];
        let mut strings = vec![];
        name.extend(0u16.to_be_bytes());
        name.extend((names.len() as u16).to_be_bytes());
        name.extend((6 + 12 * names.len() as u16).to_be_bytes());
        for (language, id, string) in names {
            let bytes = string
                .encode_utf16()
                .flat_map(u16::to_be_bytes)
                .collect::<Vec<_>>();
            for field in [3, 1, language, id, bytes.len() as u16, strings.len() as u16] {
                name.extend(field.to_be_bytes());
            }
            strings.extend(bytes);
        }
        name.extend(strings);

        let has_postscript_names = instances.iter().any(|(_, id, _)| id.is_some());
        let instance_size = axes.len() * 4 + if has_postscript_names { 6 } else { 4 };
        let mut fvar = vec![];
        for field in [1, 0, 16, 2, axes.len(), 20, instances.len(), instance_size] {
            fvar.extend((field as u16).to_be_bytes());
        }
        for &(tag, min, default, max) in axes {
            fvar.extend(tag);
            fvar.extend([fixed(min), fixed(default), fixed(max)].concat());
            fvar.extend([0, 0, 1, 0]);
        }
        for &(subfamily, postscript_name, coordinates) in instances {
            fvar.extend(subfamily.to_be_bytes());
            fvar.extend([0, 0]);
            fvar.extend(coordinates.iter().flat_map(|value| fixed(*value)));
            if has_postscript_names {
                fvar.extend(postscript_name.unwrap_or(0xFFFF).to_be_bytes());
            }
        }

        let mut font = vec![];
        font.extend(0x00010000u32.to_be_bytes());
        font.extend([0, 2, 0, 32, 0, 1, 0, 0]);
        let fvar_offset = 12 + 2 * 16;
        let name_offset = fvar_offset + fvar.len().next_multiple_of(4);
        for (tag, offset, len) in [
            (b"fvar", fvar_offset, fvar.len()),
            (b"name", name_offset, name.len()),
        ] {
            font.extend(tag);
            font.extend(0u32.to_be_bytes());
            font.extend((offset as u32).to_be_bytes());
            font.extend((len as u32).to_be_bytes());
        }
        font.extend(&fvar);
        font.resize(name_offset, 0);
        font.extend(&name);
        font
    }

    #[test]
    fn fixed_decimal() {
        assert_eq!(fixed(Fixed::ZERO), "0");
        assert_eq!(fixed(Fixed::from_i32(900)), "900");
        assert_eq!(fixed(Fixed::from_f64(62.5)), "62.5");
        assert_eq!(fixed(Fixed::from_f64(5.5)), "5.5");
        assert_eq!(fixed(Fixed::from_f64(-1.4)), "-1.4");
        assert_eq!(fixed(Fixed::from_f64(-2.9)), "-2.9");
        assert_eq!(fixed(Fixed::from_f64(0.25)), ".25");
        assert_eq!(fixed(Fixed::from_bits(1)), ".00002");
        assert_eq!(fixed(Fixed::MIN), "-32768");
        assert_eq!(fixed(Fixed::MAX), "32767.99998");
    }

    #[test]
    fn fixed_decimal_round_trip() {
        for int_part in [0, 1, -1, 400, -32768] {
            for frac_part in 0..0x10000 {
                let bits = (int_part << 16) | frac_part;
                let string = fixed(Fixed::from_bits(bits));
                let value: f64 = string.parse().unwrap();
                assert_eq!((value * 65536.0).round() as i32, bits, "{string}");
            }
        }
    }

    #[test]
    fn default_instance() {
        assert_eq!(name(NOTO_SANS_VAR.as_ref(), &[]), None);
        assert_eq!(name(NOTO_SANS_VAR.as_ref(), &[(b"wght", 400.0)]), None);
        assert_eq!(name(NOTO_SANS_VAR.as_ref(), &[(b"wdth", 120.0)]), None);
        assert_eq!(name(NOTO_SANS_VAR.as_ref(), &[(b"slnt", -10.0)]), None);
    }

    #[test]
    fn named_instance() {
        assert_eq!(
            name(NOTO_SANS_VAR.as_ref(), &[(b"wght", 100.0)]).as_deref(),
            Some("NotoSans-Thin")
        );
        assert_eq!(
            name(NOTO_SANS_VAR.as_ref(), &[(b"wght", 900.0), (b"wdth", 62.5)]).as_deref(),
            Some("NotoSans-ExtraCondensedBlack")
        );
        // Values are clamped, and the last value for an axis is used.
        assert_eq!(
            name(
                NOTO_SANS_VAR.as_ref(),
                &[(b"wght", 300.0), (b"wght", 1000.0)]
            )
            .as_deref(),
            Some("NotoSans-Black")
        );
        assert_eq!(
            name(CANTARELL_VAR.as_ref(), &[(b"wght", 800.0)]).as_deref(),
            Some("Cantarell-ExtraBold")
        );
    }

    #[test]
    fn arbitrary_instance() {
        assert_eq!(
            name(NOTO_SANS_VAR.as_ref(), &[(b"wght", 650.0)]).as_deref(),
            Some("NotoSans_650wght")
        );
        assert_eq!(
            name(NOTO_SANS_VAR.as_ref(), &[(b"wght", 350.5), (b"wdth", 80.0)]).as_deref(),
            Some("NotoSans_350.5wght_80wdth")
        );
        assert_eq!(
            name(NOTO_SANS_VAR.as_ref(), &[(b"wdth", 70.0)]).as_deref(),
            Some("NotoSans_70wdth")
        );
    }

    #[test]
    fn long_names() {
        const WGHT: (&[u8; 4], f64, f64, f64) = (b"wght", 100.0, 400.0, 900.0);
        const WDTH: (&[u8; 4], f64, f64, f64) = (b"wdth", 50.0, 100.0, 100.0);
        let long = |family: &str| test_font(&[(0x409, 1, family)], &[WGHT, WDTH], &[]);

        // Names up to the limit are kept in full, even if the prefix alone is long.
        let one = long(&format!("{}One", "A".repeat(95)));
        let two = long(&format!("{}Two", "A".repeat(95)));
        let one = name(&one, &[(b"wght", 650.0)]).unwrap();
        let two = name(&two, &[(b"wght", 650.0)]).unwrap();
        assert_eq!(one, format!("{}One_650wght", "A".repeat(95)));
        assert_eq!(two, format!("{}Two_650wght", "A".repeat(95)));

        // Longer names are replaced by last resort names that stay distinct.
        let font = long(&"Long".repeat(25));
        let narrow = name(&font, &[(b"wght", 650.0), (b"wdth", 75.0)]).unwrap();
        let wide = name(&font, &[(b"wght", 650.0), (b"wdth", 80.0)]).unwrap();
        assert_ne!(narrow, wide);
        for name in [&narrow, &wide] {
            assert_eq!(name.len(), MAX_POSTSCRIPT_NAME_LEN);
            assert!(name.starts_with("LongLong"));
            assert!(name.ends_with("..."));
        }
    }

    #[test]
    fn postscript_name_without_family_prefix() {
        let font = test_font(
            &[
                (0x411, 1, "游ゴシック"),
                (0x409, 256, "Thin"),
                (0x409, 257, "YuGothic-Thin"),
            ],
            &[(b"wght", 100.0, 400.0, 900.0)],
            &[(256, Some(257), &[100.0])],
        );
        assert_eq!(
            name(&font, &[(b"wght", 100.0)]).as_deref(),
            Some("YuGothic-Thin")
        );
        // Without an ASCII family name, other instances can't be named.
        assert_eq!(name(&font, &[(b"wght", 650.0)]), None);
    }

    #[test]
    fn long_postscript_name() {
        let postscript_name = "Instance".repeat(20);
        let font = test_font(
            &[
                (0x409, 1, "Family"),
                (0x409, 256, "Thin"),
                (0x409, 257, &postscript_name),
            ],
            &[(b"wght", 100.0, 400.0, 900.0)],
            &[(256, Some(257), &[100.0])],
        );
        let name = name(&font, &[(b"wght", 100.0)]).unwrap();
        assert_eq!(name.len(), MAX_POSTSCRIPT_NAME_LEN);
        assert!(name.starts_with("InstanceInstance"));
    }
}
