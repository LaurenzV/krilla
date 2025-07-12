use std::any::Any;
use std::fmt;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

use base64::Engine;
pub(crate) use deferred::*;
use pdf_writer::{Dict, Name};
use siphasher::sip128::{Hasher128, SipHasher13};
use tiny_skia_path::Path;

use crate::geom::Rect;
use crate::graphics::color::{DEVICE_CMYK, DEVICE_GRAY, DEVICE_RGB};
use crate::graphics::paint::Stroke;
use crate::resource::Resource;
use crate::serialize::MaybeDeviceColorSpace;

pub(crate) trait NameExt {
    fn to_pdf_name(&self) -> Name;
}

impl NameExt for String {
    fn to_pdf_name(&self) -> Name {
        Name(self.as_bytes())
    }
}

impl NameExt for &str {
    fn to_pdf_name(&self) -> Name {
        Name(self.as_bytes())
    }
}

pub(crate) fn calculate_stroke_bbox(stroke: &Stroke, path: &Path) -> Option<Rect> {
    let stroke = stroke.clone().into_tiny_skia();

    if let Some(stroked_path) = path.stroke(&stroke, 1.0) {
        return Some(Rect::from_tsp(stroked_path.compute_tight_bounds()?));
    }

    None
}

pub(crate) struct Prehashed<T: ?Sized> {
    hash: u128,
    value: T,
}

impl<T: Hash + 'static> Prehashed<T> {
    #[inline]
    pub fn new(value: T) -> Self {
        let hash = value.sip_hash();
        Self { hash, value }
    }
}

impl<T: Hash + ?Sized + 'static> Eq for Prehashed<T> {}

impl<T: Hash + ?Sized + 'static> PartialEq for Prehashed<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl<T: ?Sized> Deref for Prehashed<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Hash + Clone + 'static> Clone for Prehashed<T> {
    fn clone(&self) -> Self {
        Self {
            hash: self.hash,
            value: self.value.clone(),
        }
    }
}

impl<T: Debug> Debug for Prehashed<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<T: Hash + ?Sized + 'static> Hash for Prehashed<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u128(self.hash);
    }
}

/// Extra methods for [`[T]`](slice).
pub(crate) trait SliceExt<T> {
    /// Split a slice into consecutive runs with the same key and yield for
    /// each such run the key and the slice of elements with that key.
    fn group_by_key<K, F>(&self, f: F) -> GroupByKey<'_, T, F>
    where
        F: FnMut(&T) -> K,
        K: PartialEq;
}

impl<T> SliceExt<T> for [T] {
    fn group_by_key<K, F>(&self, f: F) -> GroupByKey<'_, T, F> {
        GroupByKey { slice: self, f }
    }
}

/// This struct is created by [`SliceExt::group_by_key`].
pub(crate) struct GroupByKey<'a, T, F> {
    slice: &'a [T],
    f: F,
}

impl<'a, T, K, F> Iterator for GroupByKey<'a, T, F>
where
    F: FnMut(&T) -> K,
    K: PartialEq,
{
    type Item = (K, &'a [T]);

    fn next(&mut self) -> Option<Self::Item> {
        let mut iter = self.slice.iter();
        let key = (self.f)(iter.next()?);
        let count = 1 + iter.take_while(|t| (self.f)(t) == key).count();
        let (head, tail) = self.slice.split_at(count);
        self.slice = tail;
        Some((key, head))
    }
}

pub(crate) trait SipHashable {
    fn sip_hash(&self) -> u128;
}

impl<T> SipHashable for T
where
    T: Hash + ?Sized + 'static,
{
    fn sip_hash(&self) -> u128 {
        let mut state = SipHasher13::new();
        self.type_id().hash(&mut state);
        self.hash(&mut state);
        state.finish128().as_u128()
    }
}

/// Create a base64-encoded hash of the value.
pub(crate) fn hash_base64<T: Hash + ?Sized>(value: &T) -> String {
    base64::engine::general_purpose::STANDARD.encode(hash128(value).to_be_bytes())
}

/// Calculate a 128-bit siphash of a value.
pub(crate) fn hash128<T: Hash + ?Sized>(value: &T) -> u128 {
    let mut state = SipHasher13::new();
    value.hash(&mut state);
    state.finish128().as_u128()
}

pub(crate) fn set_colorspace(cs: MaybeDeviceColorSpace, target: &mut Dict) {
    let pdf_cs = target.insert(Name(b"ColorSpace"));

    match cs {
        MaybeDeviceColorSpace::DeviceGray => pdf_cs.primitive(DEVICE_GRAY.to_pdf_name()),
        MaybeDeviceColorSpace::DeviceRgb => pdf_cs.primitive(DEVICE_RGB.to_pdf_name()),
        MaybeDeviceColorSpace::DeviceCMYK => pdf_cs.primitive(DEVICE_CMYK.to_pdf_name()),
        MaybeDeviceColorSpace::ColorSpace(cs) => pdf_cs.primitive(cs.get_ref()),
    }
}

pub(crate) mod lazy {
    /// Lazily initialize a value with a consuming function.
    /// This allows producing a value from a mutable reference.
    pub struct LazyInit<U, I, F> {
        inner: Option<Repr<U, I>>,
        f: F,
    }

    enum Repr<U, I> {
        Uninit(U),
        Init(I),
    }

    impl<U, I, F> LazyInit<U, I, F>
    where
        F: Fn(U) -> I,
    {
        pub fn new(uninit: U, f: F) -> Self {
            Self {
                inner: Some(Repr::Uninit(uninit)),
                f,
            }
        }

        // Using DerefMut would be nice, but that would require a Deref impl,
        // which is not possible because initialization requires mutable access.
        pub fn get(&mut self) -> &mut I {
            let init = match self.inner.take().expect("inner to be present") {
                Repr::Uninit(uninit) => (self.f)(uninit),
                Repr::Init(init) => init,
            };
            match self.inner.insert(Repr::Init(init)) {
                Repr::Uninit(_) => unreachable!(),
                Repr::Init(init) => init,
            }
        }
    }
}

#[cfg(not(feature = "rayon"))]
mod deferred {
    pub(crate) struct Deferred<T>(T);

    impl<T: Send + Sync + 'static> Deferred<T> {
        pub fn new<F>(f: F) -> Self
        where
            F: FnOnce() -> T + Send + Sync + 'static,
        {
            Self(f())
        }

        pub fn wait(&self) -> &T {
            &self.0
        }
    }
}

#[cfg(feature = "rayon")]
mod deferred {
    /// A value that is lazily executed on another thread.
    ///
    /// Execution will be started in the background and can be waited on.
    pub(crate) struct Deferred<T>(std::sync::Arc<once_cell::sync::OnceCell<T>>);

    #[cfg(feature = "rayon")]
    impl<T: Send + Sync + 'static> Deferred<T> {
        /// Creates a new deferred value.
        ///
        /// The closure will be called on a secondary thread such that the value
        /// can be initialized in parallel.
        pub fn new<F>(f: F) -> Self
        where
            F: FnOnce() -> T + Send + Sync + 'static,
        {
            let inner = std::sync::Arc::new(once_cell::sync::OnceCell::new());
            let cloned = std::sync::Arc::clone(&inner);
            rayon::spawn(move || {
                // Initialize the value if it hasn't been initialized yet.
                // We do this to avoid panicking in case it was set externally.
                cloned.get_or_init(f);
            });
            Self(inner)
        }

        /// Waits on the value to be initialized.
        ///
        /// If the value has already been initialized, this will return
        /// immediately. Otherwise, this will block until the value is
        /// initialized in another thread.
        pub fn wait(&self) -> &T {
            // Fast path if the value is already available. We don't want to yield
            // to rayon in that case.
            if let Some(value) = self.0.get() {
                return value;
            }

            // Ensure that we yield to give the deferred value a chance to compute
            // single-threaded platforms (for WASM compatibility).
            while let Some(rayon::Yield::Executed) = rayon::yield_now() {}

            self.0.wait()
        }
    }
}

#[cfg(test)]
pub(crate) mod test_utils {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::LazyLock;

    use once_cell::sync::Lazy;

    use crate::Data;

    pub(crate) static WORKSPACE_PATH: Lazy<PathBuf> =
        Lazy::new(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../"));

    pub(crate) static ASSETS_PATH: LazyLock<PathBuf> =
        LazyLock::new(|| WORKSPACE_PATH.join("assets"));

    static FONT_PATH: LazyLock<PathBuf> = LazyLock::new(|| ASSETS_PATH.join("fonts"));

    macro_rules! lazy_font {
        ($name:ident, $path:expr) => {
            pub static $name: LazyLock<Data> =
                LazyLock::new(|| Arc::new(std::fs::read($path).unwrap()).into());
        };
    }

    lazy_font!(
        NOTO_COLOR_EMOJI_COLR,
        FONT_PATH.join("NotoColorEmoji.COLR.subset.ttf")
    );
}
