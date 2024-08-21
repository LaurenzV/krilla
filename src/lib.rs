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
pub use paint::*;

pub use path::*;
pub use tiny_skia_path::{Size, Transform};

// TODO: Add acknowledgements and license files

#[cfg(test)]
pub(crate) mod test_utils {
    use difference::{Changeset, Difference};
    use std::path::PathBuf;

    const REPLACE: bool = false;

    pub fn load_font(name: &str) -> Vec<u8> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fonts")
            .join(name);
        std::fs::read(&path).unwrap()
    }

    fn snapshot_path(name: &str) -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots");

        let parts = name.split("/").collect::<Vec<_>>();

        for i in 0..parts.len() - 1 {
            path.push(parts[i]);
        }

        std::fs::create_dir_all(&path).unwrap();

        path.push(format!("{}.txt", parts.last().unwrap()));
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
}
