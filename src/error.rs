pub type KrillaResult<T> = Result<T, KrillaError>;

#[derive(Debug)]
pub enum KrillaError {
    Font(String),
    GlyphDrawing(String),
}
