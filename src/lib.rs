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
pub use object::mask::MaskType;
pub use paint::*;

pub use path::*;
pub use tiny_skia_path::{Size, Transform};

#[cfg(test)]
mod test_utils {
    use difference::{Changeset, Difference};
    use std::path::{Path, PathBuf};

    const REPLACE: bool = false;

    fn snapshot_path(name: &str) -> PathBuf {
        let mut path = PathBuf::new();
        path.push(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/snapshots");
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

        let mut t = term::stdout().unwrap();

        for diff in changeset.diffs {
            match diff {
                Difference::Same(ref x) => {
                    t.reset().unwrap();
                    writeln!(t, " {}", x);
                }
                Difference::Add(ref x) => {
                    t.fg(term::color::GREEN).unwrap();
                    writeln!(t, "+++++++++++++++++++\n{}\n+++++++++++++++++++", x);
                }
                Difference::Rem(ref x) => {
                    t.fg(term::color::RED).unwrap();
                    writeln!(t, "-------------------\n{}\n-------------------", x);
                }
            }
        }
        t.reset().unwrap();
        t.flush().unwrap();

        assert_eq!(changeset.distance, 0);
    }
}
