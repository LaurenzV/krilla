pub mod chunk_container;
pub mod document;
pub mod font;
mod graphics_state;
mod object;
pub mod paint;
pub mod path;
pub mod resource;
pub mod serialize;
pub mod stream;
pub mod surface;
pub mod svg;
pub mod transform;
pub mod util;

pub use fontdb::*;
pub use object::color_space::rgb;
pub use object::mask::MaskType;
pub use object::*;
pub use paint::*;

pub use path::*;
pub use tiny_skia_path::{Size, Transform};

// TODO: Add acknowledgements and license files

#[cfg(test)]
pub(crate) mod test_utils {
    use crate::font::Font;
    use crate::stream::Glyph;
    use difference::{Changeset, Difference};
    use rustybuzz::{Direction, UnicodeBuffer};
    use skrifa::GlyphId;
    use std::path::PathBuf;
    use tiny_skia_path::{Path, PathBuilder, Rect};

    const REPLACE: bool = true;

    pub fn rect_path(x1: f32, y1: f32, x2: f32, y2: f32) -> Path {
        let mut builder = PathBuilder::new();
        builder.push_rect(Rect::from_ltrb(x1, y1, x2, y2).unwrap());
        builder.finish().unwrap()
    }

    pub fn load_font(name: &str) -> Vec<u8> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fonts")
            .join(name);
        std::fs::read(&path).unwrap()
    }

    fn snapshot_path(name: &str) -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots");

        std::fs::create_dir_all(&path).unwrap();

        path.push(format!("{}.txt", name));
        path
    }

    pub fn check_snapshot(name: &str, content: &[u8]) {
        let path = snapshot_path(name);

        if !path.exists() {
            std::fs::write(path, &content).unwrap();
            panic!("new snapshot created");
        }

        let actual = std::fs::read(&path).unwrap();

        if REPLACE && &actual != content {
            std::fs::write(&path, content).unwrap();
            panic!("test was replaced");
        }

        let changeset = Changeset::new(
            &String::from_utf8_lossy(content),
            &String::from_utf8_lossy(&actual),
            "\n",
        );

        for diff in changeset.diffs {
            match diff {
                Difference::Same(ref x) => {
                    println!(" {}", x);
                }
                Difference::Add(ref x) => {
                    println!("+++++++++++++++++++\n{}\n+++++++++++++++++++", x);
                }
                Difference::Rem(ref x) => {
                    println!("-------------------\n{}\n-------------------", x);
                }
            }
        }

        assert_eq!(changeset.distance, 0);
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
}
