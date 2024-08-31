pub type KrillaResult<T> = Result<T, KrillaError>;

#[derive(Debug, PartialEq, Eq)]
pub enum KrillaError {
    Font(String),
    GlyphDrawing(String),
    UserError(String),
}
