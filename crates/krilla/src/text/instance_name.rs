//! PostScript names for instances of variable fonts.
//!
//! The name ID 6 of a variable font names its default instance, so it can't be used
//! for other instances. Instead, we generate a name as described in Adobe Technical
//! Note #5902, the same way FreeType does.

use std::fmt::Write;

use skrifa::raw::types::{Fixed, NameId};
use skrifa::raw::TableProvider;
use skrifa::{FontRef, MetadataProvider};

use crate::text::Tag;
use crate::util::stable_hash128;

/// The maximum length of a PostScript name.
const MAX_LEN: usize = 127;
/// The length of a hyphen, a 128-bit hash in hexadecimal and three periods,
/// which follow the family prefix in a last resort name.
const LAST_RESORT_LEN: usize = 1 + 32 + 3;

/// Generate the PostScript name of the instance of a variable font at the given
/// variation coordinates.
///
/// Returns `None` if the coordinates select the default instance, which is named by
/// name ID 6, or if the font has no family name to derive a name from.
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

    let mut prefix = [
        NameId::VARIATIONS_POSTSCRIPT_NAME_PREFIX,
        NameId::TYPOGRAPHIC_FAMILY_NAME,
        NameId::FAMILY_NAME,
    ]
    .into_iter()
    .find_map(|id| name_string(font_ref, id))
    .map(alphanumeric)?;
    // Leave room for the identifier of a last resort name.
    prefix.truncate(MAX_LEN - LAST_RESORT_LEN);
    if prefix.is_empty() {
        return None;
    }

    let named_instance = fvar.instances().ok()?.iter().find_map(|instance| {
        let instance = instance.ok()?;
        let coordinates = instance.coordinates.iter().map(|c| c.get());
        coordinates.eq(coords.iter().copied()).then_some(instance)
    });

    let named_instance_name = named_instance.and_then(|instance| {
        instance
            .post_script_name_id
            .and_then(|id| name_string(font_ref, id))
            .filter(|name| !name.is_empty())
            .or_else(|| {
                let subfamily = alphanumeric(name_string(font_ref, instance.subfamily_name_id)?);
                (!subfamily.is_empty()).then(|| format!("{prefix}-{subfamily}"))
            })
    });

    let name = named_instance_name.unwrap_or_else(|| {
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
    });

    if name.len() > MAX_LEN {
        return Some(format!("{prefix}-{:032X}...", stable_hash128(&name)));
    }

    Some(name)
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
    use crate::text::Tag;
    use crate::util::test_utils::{CANTARELL_VAR, NOTO_SANS_VAR};
    use crate::Data;

    fn fixed(value: Fixed) -> String {
        let mut name = String::new();
        write_fixed(&mut name, value);
        name
    }

    fn name(data: &Data, coords: &[(&[u8; 4], f32)]) -> Option<String> {
        let font_ref = FontRef::new(data.as_ref()).unwrap();
        let coords = coords
            .iter()
            .map(|(tag, value)| (Tag::new(tag), *value))
            .collect::<Vec<_>>();
        instance_postscript_name(&font_ref, &coords)
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
        assert_eq!(name(&NOTO_SANS_VAR, &[]), None);
        assert_eq!(name(&NOTO_SANS_VAR, &[(b"wght", 400.0)]), None);
        assert_eq!(name(&NOTO_SANS_VAR, &[(b"wdth", 120.0)]), None);
        assert_eq!(name(&NOTO_SANS_VAR, &[(b"slnt", -10.0)]), None);
    }

    #[test]
    fn named_instance() {
        assert_eq!(
            name(&NOTO_SANS_VAR, &[(b"wght", 100.0)]).as_deref(),
            Some("NotoSans-Thin")
        );
        assert_eq!(
            name(&NOTO_SANS_VAR, &[(b"wght", 900.0), (b"wdth", 62.5)]).as_deref(),
            Some("NotoSans-ExtraCondensedBlack")
        );
        // Values are clamped, and the last value for an axis is used.
        assert_eq!(
            name(&NOTO_SANS_VAR, &[(b"wght", 300.0), (b"wght", 1000.0)]).as_deref(),
            Some("NotoSans-Black")
        );
        assert_eq!(
            name(&CANTARELL_VAR, &[(b"wght", 800.0)]).as_deref(),
            Some("Cantarell-ExtraBold")
        );
    }

    #[test]
    fn arbitrary_instance() {
        assert_eq!(
            name(&NOTO_SANS_VAR, &[(b"wght", 650.0)]).as_deref(),
            Some("NotoSans_650wght")
        );
        assert_eq!(
            name(&NOTO_SANS_VAR, &[(b"wght", 350.5), (b"wdth", 80.0)]).as_deref(),
            Some("NotoSans_350.5wght_80wdth")
        );
        assert_eq!(
            name(&NOTO_SANS_VAR, &[(b"wdth", 70.0)]).as_deref(),
            Some("NotoSans_70wdth")
        );
    }
}
