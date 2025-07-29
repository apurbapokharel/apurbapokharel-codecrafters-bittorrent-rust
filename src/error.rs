use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("File error")]
    FileError,
    
    #[error("Invalid input")]
    InvalidInput,

    #[error("Pieces not divisible by 20")]
    PiecesLengthNotDivisible
}