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
    use std::path::{Path, PathBuf};

    const REPLACE: bool = true;

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

        assert!(&actual == content);
    }
}
