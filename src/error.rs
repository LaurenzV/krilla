pub type KrillaResult<T> = Result<T, KrillaError>;

#[derive(Debug)]
pub enum KrillaError {
    FontError(String),
}
