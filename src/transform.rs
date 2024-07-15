use tiny_skia_path::{FiniteF32, Transform};

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct FiniteTransform {
    pub sx: FiniteF32,
    pub kx: FiniteF32,
    pub ky: FiniteF32,
    pub sy: FiniteF32,
    pub tx: FiniteF32,
    pub ty: FiniteF32,
}

impl Into<Transform> for FiniteTransform {
    fn into(self) -> Transform {
        Transform {
            sx: self.sx.get(),
            kx: self.kx.get(),
            ky: self.ky.get(),
            sy: self.sy.get(),
            tx: self.tx.get(),
            ty: self.ty.get(),
        }
    }
}

impl TryFrom<Transform> for FiniteTransform {
    type Error = ();

    fn try_from(value: Transform) -> Result<Self, Self::Error> {
        Ok(FiniteTransform {
            sx: FiniteF32::new(value.sx).ok_or(())?,
            kx: FiniteF32::new(value.kx).ok_or(())?,
            ky: FiniteF32::new(value.ky).ok_or(())?,
            sy: FiniteF32::new(value.sy).ok_or(())?,
            tx: FiniteF32::new(value.tx).ok_or(())?,
            ty: FiniteF32::new(value.ty).ok_or(())?,
        })
    }
}
