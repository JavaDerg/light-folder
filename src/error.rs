use std::fmt::{Display, Formatter};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    OneshotReceiveError,
    SaphirError(saphir::error::SaphirError),
    ImageError(ImageError),
}

#[derive(Debug)]
pub enum ImageError {
    ImageEncodingError(String),
    ImageLoadingError(String),
    GeneralImageError(String),
    ImageCreationError(String),
    ImageResizingError(String),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}

impl From<ImageError> for Error {
    fn from(err: ImageError) -> Self {
        Error::ImageError(err)
    }
}
