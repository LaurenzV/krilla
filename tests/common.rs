use std::path::PathBuf;
use rustybuzz::{Direction, UnicodeBuffer};
use skrifa::GlyphId;
use krilla::font::Font;
use krilla::serialize::{SerializeSettings, SvgSettings};
use krilla::stream::Glyph;

pub trait SerializeSettingsExt {
    fn settings_1() -> Self;
    fn settings_2() -> Self;
}

impl SerializeSettingsExt for SerializeSettings {
    fn settings_1() -> Self {
        Self {
            ascii_compatible: true,
            compress_content_streams: false,
            no_device_cs: false,
            force_type3_fonts: false,
            svg_settings: SvgSettings::default(),
        }
    }

    fn settings_2() -> Self {
        Self {
            no_device_cs: true,
            ..Self::settings_1()
        }
    }
}

pub fn store(name: &str, pdf: Vec<u8>) {
    let _ = std::fs::write(format!("out/simple_shape_demo.pdf"), &pdf);
    let _ = std::fs::write(format!("out/simple_shape_demo.txt"), &pdf);
}

pub fn simple_shape(text: &str, dir: Direction, font: Font, size: f32) -> Vec<Glyph> {
    let data = font.font_data();
    let rb_font = rustybuzz::Face::from_slice(data.as_ref().as_ref(), 0).unwrap();

    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_direction(dir);

    let output = rustybuzz::shape(&rb_font, &[], buffer);

    let positions = output.glyph_positions();
    let infos = output.glyph_infos();

    let mut glyphs = vec![];

    for i in 0..output.len() {
        let pos = positions[i];
        let start_info = infos[i];

        let start = start_info.cluster as usize;

        let end = if dir == Direction::LeftToRight {
            let mut e = i.checked_add(1);
            loop {
                if let Some(index) = e {
                    if let Some(end_info) = infos.get(index) {
                        if end_info.cluster == start_info.cluster {
                            e = index.checked_add(1);
                            continue;
                        }
                    }
                }

                break;
            }

            e
        } else {
            let mut e = i.checked_sub(1);
            loop {
                if let Some(index) = e {
                    if let Some(end_info) = infos.get(index) {
                        if end_info.cluster == start_info.cluster {
                            e = index.checked_sub(1);
                        } else {
                            break;
                        }
                    }
                } else {
                    break;
                }
            }

            e
        }
            .and_then(|last| infos.get(last))
            .map_or(text.len(), |info| info.cluster as usize);

        glyphs.push(Glyph::new(
            GlyphId::new(start_info.glyph_id),
            (pos.x_advance as f32 / font.units_per_em() as f32) * size,
            (pos.x_offset as f32 / font.units_per_em() as f32) * size,
            (pos.y_offset as f32 / font.units_per_em() as f32) * size,
            start..end,
            size,
        ));
    }

    glyphs
}

pub fn load_font(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fonts")
        .join(name);
    std::fs::read(&path).unwrap()
}